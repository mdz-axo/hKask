# K8s Deployment Architecture

How hKask resources connect in the Kubernetes cluster. Two namespaces, one Ingress, two PVC-backed pods with sidecars. Extracted from `deploy/k8s/` manifests and the admin guide.

```mermaid
flowchart TD
    User([Browser / Matrix Client])
    DNS[(DNS: hkask.example.com)]

    User --> DNS
    DNS --> Ingress

    subgraph Cluster
        Ingress[Ingress: nginx + cert-manager TLS]
        Ingress -->|"/"| KaskSvc[kask Service :3000]
        Ingress -->|"/_matrix"| ConduitBridge[conduit ExternalName Service]
        ConduitBridge -.->|cross-ns DNS| ConduitSvc[conduit Service :8008]

        subgraph ns_hkask[Namespace: hkask]
            KaskSvc --> KaskPod

            subgraph KaskPod[Pod: hkask]
                InitWfc[init: wait-for-conduit]
                InitRestore[init: litestream-restore]
                KaskContainer[kask serve]
                LitestreamSidecar[litestream replicate]
                InitWfc --> InitRestore --> KaskContainer
                LitestreamSidecar -.->|shared /data| DataPV[(PVC: hkask-data 20Gi)]
                KaskContainer --> DataPV
            end

            NP_hkask[NetworkPolicy: deny-all]
            NP_hkask -.-> KaskPod

            KaskSecrets[Secret: hkask-secrets] -.-> KaskPod
            KaskConfig[ConfigMap: hkask-config] -.-> KaskPod
            KaskPDB[PDB: maxUnavailable 0] -.-> KaskPod
        end

        subgraph ns_conduit[Namespace: hkask-conduit]
            ConduitSvc --> ConduitPod

            subgraph ConduitPod[Pod: conduit]
                ConduitContainer[conduit :8008]
                ConduitContainer --> ConduitData[(PVC: conduit-data 10Gi)]
            end

            NP_conduit[NetworkPolicy: deny-all] -.-> ConduitPod
            ConduitSecrets[Secret: conduit-secrets] -.-> ConduitPod
        end

        KaskContainer -->|HTTP Matrix API| ConduitSvc
    end

    LitestreamSidecar -->|WAL replication| S3[(S3 Object Storage)]
    InitRestore -->|restore from| S3
```

**Readiness flow:** `GET /health` → DB query + Conduit reachability → 200 if both OK, 503 otherwise. K8s readiness probe uses this.
**Liveness flow:** `GET /` → static HTML → always 200 (fast, only proves HTTP server is alive).

For the startup sequence, see `docs/diagrams/flowchart-pod-startup.md`.
For resource relationships, see `docs/diagrams/erd-k8s-resources.md`.
For the full admin guide, see `docs/plans/k8s-admin-guide.md`.
