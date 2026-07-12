#!/usr/bin/env python3
"""
Capabilities Researcher LoRA Training — Qwen3.6-27B via Unsloth
Trains on the capabilities-researcher-qa dataset from HuggingFace.
"""
import os, torch
from unsloth import FastLanguageModel
from datasets import load_dataset
from trl import SFTTrainer, SFTConfig

# ── Config ──────────────────────────────────────────────────────────
BASE_MODEL = "unsloth/Qwen3.6-27B"
DATASET = "Axolotl-Partners/capabilities-researcher-qa"
ADAPTER_DIR = "/workspace/adapter"
MAX_SEQ = 4096
LORA_R = 64
LORA_ALPHA = 64
LORA_DROPOUT = 0
LR = 2e-4
EPOCHS = 3
BATCH_SIZE = 1
GRAD_ACCUM = 4
WARMUP_STEPS = 100

print(f"Loading {BASE_MODEL}...")
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=BASE_MODEL,
    max_seq_length=MAX_SEQ,
    dtype=torch.bfloat16,
    load_in_4bit=False,
    load_in_8bit=False,
)
print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB allocated")

model = FastLanguageModel.get_peft_model(
    model,
    r=LORA_R,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    lora_alpha=LORA_ALPHA,
    lora_dropout=LORA_DROPOUT,
    bias="none",
    use_gradient_checkpointing="unsloth",
    random_state=42,
)

print("Loading dataset...")
dataset = load_dataset(DATASET, token=os.environ.get("HF_TOKEN"))
train_ds = dataset["train"] if "train" in dataset else dataset[list(dataset.keys())[0]]
print(f"Train examples: {len(train_ds)}")

# Apply chat template
def format_example(ex):
    messages = ex["messages"]
    text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=False)
    return {"text": text}

train_ds = train_ds.map(format_example, remove_columns=train_ds.column_names)
print(f"Sample: {train_ds[0]['text'][:200]}...")

config = SFTConfig(
    output_dir=ADAPTER_DIR,
    num_train_epochs=EPOCHS,
    per_device_train_batch_size=BATCH_SIZE,
    gradient_accumulation_steps=GRAD_ACCUM,
    warmup_steps=WARMUP_STEPS,
    learning_rate=LR,
    lr_scheduler_type="cosine",
    optim="adamw_8bit",
    weight_decay=0.01,
    max_grad_norm=0.3,
    bf16=True,
    logging_steps=10,
    save_steps=200,
    save_total_limit=3,
    report_to="none",
    dataset_text_field="text",
    max_seq_length=MAX_SEQ,
)

print("Starting training...")
trainer = SFTTrainer(
    model=model,
    train_dataset=train_ds,
    args=config,
)
trainer.train()

print("Saving adapter...")
model.save_pretrained(ADAPTER_DIR)
tokenizer.save_pretrained(ADAPTER_DIR)
print(f"Adapter saved to {ADAPTER_DIR}")
print("Done!")
