#!/usr/bin/env python3
"""Tinker SFT for the Capabilities Researcher persona — ad-hoc test script.

Compares Tinker (Thinking Machines) against our Axolotl+PiSSA RunPod run
(mdz-axo/capabilities-researcher-lora, eval_loss 1.334, 55.5h, ~$177).
Same dataset, same rank, same effective batch, same LR; 2 epochs (we
learned epoch 2 was the peak).

Run:
  TINKER_API_KEY=... HF_TOKEN=... \
    python3 corpus/tinker_train_capabilities_researcher.py

DELETE AFTER TESTING — AGENTS.md: ad-hoc scripts must be removed before
work is complete.
"""
import asyncio, json, math, os, sys, time
from typing import List, Tuple

import tinker
from tinker import types
from huggingface_hub import hf_hub_download

# ── Config ─────────────────────────────────────────────────────────────
BASE_MODEL = "Qwen/Qwen3.6-27B"          # Tinker ID (dense, 27B, 64K ctx)
LORA_RANK = 32                            # match Axolotl PiSSA config
DATASET_REPO = "mdz-axo/capabilities-researcher-qa"
DATASET_FILE = "train_chat_full.jsonl"
EPOCHS = 2
MICRO_BATCH = 1
GRAD_ACCUM = 16                           # effective batch = 16
EVAL_EVERY = 200                          # optimizer steps
EVAL_SIZE = 64                            # held-out tail for eval
EARLY_STOP_PATIENCE = 25                  # consecutive non-improving evals
BASE_LR = 1e-4
WARMUP_STEPS = 100
MAX_SEQ_LEN = 4096


def load_dataset() -> Tuple[List[dict], List[dict]]:
    path = hf_hub_download(
        repo_id=DATASET_REPO, filename=DATASET_FILE,
        repo_type="dataset", token=os.environ["HF_TOKEN"],
    )
    examples: List[dict] = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))
    if not examples:
        sys.exit(f"No examples loaded from {DATASET_REPO}/{DATASET_FILE}")
    eval_set, train_set = examples[-EVAL_SIZE:], examples[:-EVAL_SIZE]
    print(f"Loaded {len(train_set)} train, {len(eval_set)} eval", flush=True)
    return train_set, eval_set


def encode_example(tokenizer, messages) -> Tuple[List[int], List[float], List[int]]:
    """Tokenize ChatML; loss weight 1 on assistant tokens, 0 on prompt."""
    prompt_str = "".join(f"<|im_start|>{m['role']}\n{m['content']}<|im_end|>\n" for m in messages[:-1]) + "<|im_start|>assistant\n"
    completion_str = f"{messages[-1]['content']}<|im_end|>\n"

    prompt_tokens = tokenizer.encode(prompt_str)
    completion_tokens = tokenizer.encode(completion_str)
    input_tokens = (prompt_tokens + completion_tokens)[:MAX_SEQ_LEN]

    p_len = len(prompt_tokens)
    c_eff = max(0, min(len(completion_tokens), MAX_SEQ_LEN - p_len))
    target_tokens = input_tokens[1:]
    weights = [0.0] * max(0, p_len - 1) + [1.0] * c_eff
    if len(weights) < len(target_tokens):
        weights += [0.0] * (len(target_tokens) - len(weights))
    weights = weights[: len(target_tokens)]
    return input_tokens, weights, target_tokens


def cosine_lr(step: int, total_steps: int) -> float:
    if step < WARMUP_STEPS:
        return BASE_LR * step / max(1, WARMUP_STEPS)
    progress = (step - WARMUP_STEPS) / max(1, total_steps - WARMUP_STEPS)
    return BASE_LR * 0.5 * (1.0 + math.cos(math.pi * min(1.0, progress)))


async def evaluate(sampling, tokenizer, eval_set) -> float:
    """Mean per-token NLL over completion tokens, via prompt logprobs."""
    losses: List[float] = []
    for ex in eval_set:
        input_tokens, weights, _ = encode_example(tokenizer, ex["messages"])
        prompt = types.ModelInput.from_ints(tokens=input_tokens)
        try:
            res = await sampling.sample_async(
                prompt=prompt, num_samples=1,
                sampling_params=types.SamplingParams(max_tokens=1),
                include_prompt_logprobs=True,
            )
        except Exception as e:  # noqa: BLE001
            print(f"  eval sample failed: {e}", flush=True)
            continue
        lps = res.prompt_logprobs  # lps[i] = logP(input[i] | input[:i]); lps[0]=None
        comp = [lps[j] for j in range(1, len(lps))
                if j - 1 < len(weights) and weights[j - 1] > 0
                and lps[j] is not None]
        if comp:
            losses.append(-sum(comp) / len(comp))
    return sum(losses) / len(losses) if losses else float("nan")


def log_metric(d: dict) -> None:
    print(json.dumps(d), flush=True)


async def main() -> None:
    for k in ("TINKER_API_KEY", "HF_TOKEN"):
        if not os.environ.get(k):
            sys.exit(f"Missing env var: {k}")

    train_set, eval_set = load_dataset()
    steps_per_epoch = len(train_set) // (MICRO_BATCH * GRAD_ACCUM)
    total_steps = steps_per_epoch * EPOCHS

    service = tinker.ServiceClient()
    training = service.create_lora_training_client(base_model=BASE_MODEL, rank=LORA_RANK)
    tokenizer = training.get_tokenizer()
    print(f"Base: {BASE_MODEL} | rank={LORA_RANK} | total_steps≈{total_steps}", flush=True)
    log_metric({"event": "start", "train_size": len(train_set),
                "eval_size": len(eval_set), "total_steps": total_steps})

    best_eval, best_step, no_improve, step = float("inf"), 0, 0, 0
    t0 = time.time()

    for epoch in range(EPOCHS):
        micro_batch: List[types.Datum] = []
        accum = 0
        for ex in train_set:
            input_tokens, weights, target_tokens = encode_example(tokenizer, ex["messages"])
            micro_batch.append(types.Datum(
                model_input=types.ModelInput.from_ints(tokens=input_tokens),
                loss_fn_inputs=dict(weights=weights, target_tokens=target_tokens),
            ))
            if len(micro_batch) < MICRO_BATCH:
                continue
            try:
                fwdbwd = await training.forward_backward_async(data=micro_batch, loss_fn="cross_entropy")
                result = await fwdbwd.result_async()
            except Exception as e:  # noqa: BLE001
                print(f"fwdbwd failed at step {step}: {e}", flush=True)
                micro_batch, accum = [], 0
                continue
            train_loss = result.loss
            micro_batch = []
            accum += 1
            if accum < GRAD_ACCUM:
                continue
            lr = cosine_lr(step, total_steps)
            try:
                optim = await training.optim_step_async(types.AdamParams(learning_rate=lr))
                await optim.result_async()
            except Exception as e:  # noqa: BLE001
                print(f"optim_step failed at step {step}: {e}", flush=True)
                continue
            step += 1
            accum = 0
            log_metric({"event": "step", "step": step, "epoch": epoch,
                        "train_loss": train_loss, "lr": lr})

            if step % EVAL_EVERY == 0:
                try:
                    sampling = training.save_weights_and_get_sampling_client(name=f"ckpt-{step}")
                    eval_loss = await evaluate(sampling, tokenizer, eval_set)
                except Exception as e:  # noqa: BLE001
                    print(f"eval failed at step {step}: {e}", flush=True)
                    eval_loss = float("nan")
                if eval_loss < best_eval - 1e-4:
                    best_eval, best_step, no_improve = eval_loss, step, 0
                    try:
                        training.save_state(name="best")
                    except Exception:  # noqa: BLE001
                        pass
                else:
                    no_improve += 1
                log_metric({"event": "eval", "step": step, "epoch": epoch,
                            "eval_loss": eval_loss, "best_eval": best_eval,
                            "no_improve": no_improve})
                if no_improve >= EARLY_STOP_PATIENCE:
                    log_metric({"event": "early_stop", "step": step, "no_improve": no_improve})
                    return await _finalize(training, best_eval, best_step, step, t0)

    await _finalize(training, best_eval, best_step, step, t0)


async def _finalize(training, best_eval, best_step, step, t0) -> None:
    try:
        training.save_state(name="final")
        training.save_weights_and_get_sampling_client(name="final-weights")
    except Exception as e:  # noqa: BLE001
        print(f"final save failed: {e}", flush=True)
    log_metric({"event": "summary", "best_eval": best_eval, "best_step": best_step,
                "total_steps": step, "elapsed_sec": time.time() - t0})


if __name__ == "__main__":
    asyncio.run(main())