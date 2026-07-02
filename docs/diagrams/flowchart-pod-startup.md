# K8s Pod Startup Sequence

The exact sequence when a new hKask pod is created. Understanding this helps debug startup failures. Extracted from `deploy/k8s/deployment.yaml` and the admin guide §7.

```mermaid
flowchart TD
    Start([Pod Scheduled to Node])
    Start --> Pull[Kubelet Pulls Images]
    Pull --> Init1

    subgraph InitContainers[Init Containers Sequential]
        Init1{wait-for-conduit}
        Init1 -->|"poll /_matrix/client/versions"| ConduitUp{Conduit Ready?}
        ConduitUp -->|No| Sleep[sleep 2s]
        Sleep --> ConduitUp
        ConduitUp -->|Yes| Init2

        Init2{litestream-restore}
        Init2 --> Replica{Replica in S3?}
        Replica -->|Yes| Download[Download /data/kask.db from S3]
        Replica -->|No| Skip[Skip - fresh deploy]
        Download --> InitDone
        Skip --> InitDone
    end

    InitDone([Init Complete: PodInitializing])

    InitDone --> MainStart

    subgraph MainContainers[Main Containers Parallel]
        MainStart[Main Containers Start]
        MainStart --> Kask[kask: entrypoint]
        MainStart --> Litestream[litestream: replicate]
        Kask -->|"mkdir -p /data && exec kask serve"| KaskRunning[kask serve :3000]
        Litestream -->|continuous WAL→S3| LitestreamRunning[litestream replicate]
    end

    KaskRunning --> Probes

    subgraph Probes[Health Probes]
        Liveness{K8s liveness: GET /}
        Liveness -->|200| LivenessPass[Liveness: OK]
        Liveness -->|timeout| LivenessFail[Restart Pod]
        LivenessFail -.-> Start

        Readiness{K8s readiness: GET /health}
        Readiness --> CheckDB{DB reachable?}
        CheckDB -->|No| ReadinessFail[503 - Not Ready]
        CheckDB -->|Yes| CheckConduit{Conduit reachable?}
        CheckConduit -->|No| ReadinessFail
        CheckConduit -->|Yes| ReadinessPass[200 - Ready]
        ReadinessFail -.->|retry 10s| Readiness
    end

    ReadinessPass --> Ready([Pod Ready: Service Routes Traffic])
```

The two init containers run sequentially: first `wait-for-conduit` polls until the Matrix homeserver responds, then `litestream-restore` pulls the database from S3. Main containers start in parallel. The pod is only Ready when both DB and Conduit are reachable.

For the architecture overview, see `docs/diagrams/flowchart-deployment-architecture.md`.
For the full startup sequence explanation, see `docs/plans/k8s-admin-guide.md` §7.
