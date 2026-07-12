# Training Server Class Diagram

This diagram shows the trait hierarchy and struct composition of the hKask MCP training server's provider layer. It maps the relationships between training hosts, harnesses, and parameter types.

```mermaid
classDiagram
    class TrainingHost {
        <<interface>>
        +submit(job: TrainingJob) Result~String~
        +status(job_id: String) Result~TrainingJobStatus~
        +cancel(job_id: String) Result~()~
        +list_adapters() Result~Vec~String~~
        +delete_adapter(id: String) Result~()~
        +completion_metadata(id) Result~Option~CompletionMetadata~~
        +adapter_weight_path(id) Result~Option~PathBuf~~
        +download_adapter(id, dest) Result~()~
        +estimate_cost(job) Result~CostEstimate~
    }

    class HarnessAdapter {
        <<interface>>
        +render_config(job: TrainingJob) Result~String~
        +output_dir(job_id: String) PathBuf
        +completion_marker(job_id: String) PathBuf
        +harness_id() TrainingHarnessId
    }

    class RunpodHost {
        -api_key: String
        -template_id: String
        -graphql_url: String
        -harness: Box~HarnessAdapter~
        -jobs: Arc~Mutex~HashMap~~
        +graphql_query(query, vars) Result~Value~
    }

    class TogetherHost {
        -api_key: String
        -base_url: String
    }

    class AxolotlHarness {
        +render_config(job) Result~String~
    }

    class UnslothHarness {
        +render_config(job) Result~String~
    }

    class TrainingJob {
        +id: String
        +dataset_path: PathBuf
        +base_model: String
        +params: TrainingParams
        +status: TrainingJobStatus
        +host: TrainingHostId
        +harness: TrainingHarnessId
        +estimated_cost_urj: u64
        +artifacts: Option~Artifacts~
    }

    class TrainingParams {
        +num_epochs: u32
        +batch_size: u32
        +learning_rate: f32
        +lora: LoraParams
        +quantization: QuantizationParams
        +optimization: OptimizationParams
        +sequence: SequenceParams
        +advanced: AdvancedParams
    }

    class LoraParams {
        +r: u32
        +alpha: u32
        +dropout: f32
        +target_modules: Vec~String~
        +modules_to_save: Vec~String~
        +use_rslora: bool
    }

    class OptimizationParams {
        +optimizer: Option~String~
        +weight_decay: f32
        +warmup_steps: Option~u32~
        +warmup_ratio: Option~f32~
        +lr_scheduler: Option~String~
        +gradient_accumulation_steps: u32
        +max_grad_norm: Option~f32~
    }

    class TrainingJobStatus {
        <<enumeration>>
        Queued
        Running
        Completed
        Failed
        Cancelled
    }

    class TrainingHostId {
        <<enumeration>>
        Together
        Runpod
    }

    class TrainingHarnessId {
        <<enumeration>>
        Axolotl
        Unsloth
    }

    TrainingHost <|.. RunpodHost : implements
    TrainingHost <|.. TogetherHost : implements
    HarnessAdapter <|.. AxolotlHarness : implements
    HarnessAdapter <|.. UnslothHarness : implements
    RunpodHost o-- HarnessAdapter : composes
    TrainingJob *-- TrainingParams : contains
    TrainingParams *-- LoraParams : contains
    TrainingParams *-- OptimizationParams : contains
    TrainingJob *-- TrainingJobStatus : has
    TrainingJob *-- TrainingHostId : has
    TrainingJob *-- TrainingHarnessId : has
```

## Design Notes

- `TrainingHost` is the seam for compute backends — new providers (e.g., Baseten, Modal) add without changing the router
- `HarnessAdapter` is the seam for training tooling — renders config in the harness's native format (YAML for Axolotl, Python for Unsloth)
- `RunpodHost` composes a `HarnessAdapter` — the host delegates config generation to the harness
- `TrainingParams` is a deep struct: it contains all hyperparameters as nested sub-structs, giving callers a single entry point
- `LoraParams` defaults: r=16, alpha=32, dropout=0, 7 target modules (all attention + MLP projections)
- `TrainingParams` defaults: LR=1e-4, 3 epochs, batch_size=4
