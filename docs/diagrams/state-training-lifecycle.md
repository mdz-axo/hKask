# Training Job Lifecycle State Diagram

This diagram shows the states and transitions for a training job as it moves through the hKask training pipeline. It covers both the MCP server's `TrainingJobStatus` enum and the RunPod pod lifecycle.

```mermaid
stateDiagram-v2
    [*] --> Queued : training_submit called

    Queued --> Running : Pod created on RunPod
    Queued --> Failed : Pod creation error

    Running --> Completed : Exit code 0 + adapter saved
    Running --> Failed : Exit code != 0
    Running --> Cancelled : training_cancel called

    Completed --> Uploading : HF_TOKEN present
    Completed --> SavedLocally : No HF_TOKEN

    Uploading --> Uploaded : Upload success
    Uploading --> SavedLocally : Upload failed

    Uploaded --> Terminating : Auto-terminate in 60s
    Uploaded --> Terminating : User cancels

    SavedLocally --> Terminating : User terminates manually

    Terminating --> Terminated : Pod terminated
    Terminating --> Terminated : Termination timeout

    Failed --> Terminated : User terminates
    Cancelled --> Terminated : Pod terminated

    Terminated --> [*]

    note right of Running
        Pod status mapping:
        CREATING/PENDING → Queued
        RUNNING → Running
        FAILED/ERROR/STOPPED → Failed
    end note

    note right of Completed
        Adapter artifacts:
        - adapter_model.safetensors
        - adapter_config.json
        - tokenizer.json
        - checkpoint-*/ (save_total_limit=3)
    end note
```

## State Definitions

| State | Meaning | Pod Alive? |
|-------|---------|------------|
| Queued | Job submitted, pod not yet created | No |
| Running | Pod is running, training in progress | Yes |
| Completed | Training finished successfully, adapter saved | Yes (grace period) |
| Failed | Training error or pod crash | Yes (for debugging) |
| Cancelled | User cancelled the job | Yes (until terminated) |
| Uploading | Adapter being uploaded to HF | Yes |
| Uploaded | Upload complete | Yes (60s grace) |
| SavedLocally | Adapter saved but not uploaded | Yes |
| Terminating | Pod termination in progress | Yes |
| Terminated | Pod destroyed | No |
