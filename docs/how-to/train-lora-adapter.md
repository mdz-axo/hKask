---
title: "LoRA Adapter Training Availability"
audience: [operators, developers]
last_updated: 2026-07-10
version: "0.31.1"
status: "Active"
domain: "Training"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# LoRA Adapter Training Availability

The generic CLI commands formerly described here are **not available in the current checkout**. In particular, `kask docproc ingest`, `kask training create-dataset`, `kask training start`, and `kask training status` are not implemented CLI commands. Do not use this document as an execution runbook.

For the verified state of the replica, corpus, and RunPod/Unsloth paths, see [Replica, Corpus, and Training Readiness](../status/replica-corpus-training-readiness.md).

## Current supported boundary

The repository contains MCP server code and standalone RunPod/Unsloth scripts. The MCP submission path (`hkask-mcp-training`) provides job submission, status tracking, and adapter lifecycle management, but the end-to-end contract for dataset transfer, training execution, artifact recovery, and adapter registration has not been verified through an automated integration test.

**Working training scripts** (verified on RunPod H100 NVL):

| Script | Purpose | Status |
|--------|---------|--------|
| [`train_unsloth.sh`](../../scripts/train_unsloth.sh) | Qwen3.6-27B reasoning distillation | Verified |
| [`train_rust_adapter.sh`](../../scripts/train_rust_adapter.sh) | Rust coding + analysis adapters | New |
| [`eval_unsloth.sh`](../../scripts/eval_unsloth.sh) | Adapter evaluation with baseline comparison | Verified |
| [`runpod_unsloth.sh`](../../scripts/runpod_unsloth.sh) | Pod launcher (all modes) | Verified |

See [Train Qwen3.6 on RunPod](train-qwen36-unsloth-runpod.md) and [Train Rust Adapters on RunPod](train-rust-adapters-runpod.md) for step-by-step instructions.

The smallest safe next action is to complete the MCP integration test contract, beginning with durable corpus ingestion and validated dataset production. Only a run that records the exact dataset, provider job, output adapter, and evaluation result can establish a deployable adapter through the MCP path.
