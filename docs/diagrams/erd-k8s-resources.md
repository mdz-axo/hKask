# K8s Resource Relationships

How Kubernetes resources in the hKask deployment relate to each other — what owns what, what references what. Extracted from `deploy/k8s/*.yaml`. Uses ERD notation for K8s resource dependency mapping.

```mermaid
erDiagram
    Namespace_hkask ||--|| Deployment_hkask : contains
    Namespace_hkask ||--|| Service_hkask : contains
    Namespace_hkask ||--|| Ingress_hkask : contains
    Namespace_hkask ||--|| PVC_hkask : contains
    Namespace_hkask ||--|| Secret_hkask : contains
    Namespace_hkask ||--|| ConfigMap_hkask : contains
    Namespace_hkask ||--|| NetworkPolicy_hkask : contains
    Namespace_hkask ||--|| PDB_hkask : contains
    Namespace_hkask ||--|| Service_conduit_ext : contains

    Namespace_conduit ||--|| Deployment_conduit : contains
    Namespace_conduit ||--|| Service_conduit : contains
    Namespace_conduit ||--|| PVC_conduit : contains
    Namespace_conduit ||--|| Secret_conduit : contains
    Namespace_conduit ||--|| NetworkPolicy_conduit : contains

    Deployment_hkask ||--|| Service_hkask : "selector: app=hkask"
    Deployment_hkask ||--|| PVC_hkask : "mounts /data"
    Deployment_hkask ||--|{ Secret_hkask : "env secretKeyRef"
    Deployment_hkask ||--|{ ConfigMap_hkask : "env configMapKeyRef"
    Deployment_hkask ||--o{ InitContainer : "runs sequentially"
    Deployment_hkask ||--o{ Container : "runs in parallel"

    Deployment_conduit ||--|| Service_conduit : "selector: app=conduit"
    Deployment_conduit ||--|| PVC_conduit : "mounts /data"
    Deployment_conduit ||--|{ Secret_conduit : "envFrom secretRef"

    Ingress_hkask ||--|| Service_hkask : "path: /"
    Ingress_hkask ||--|| Service_conduit_ext : "path: /_matrix"
    Service_conduit_ext ||--|| Service_conduit : "ExternalName bridge"

    PDB_hkask ||--|| Deployment_hkask : "maxUnavailable: 0"
    NetworkPolicy_hkask ||--|| Deployment_hkask : "podSelector: {}"
    NetworkPolicy_conduit ||--|| Deployment_conduit : "podSelector: {}"

    InitContainer {
        string name
        string image
        string command
    }

    Container {
        string name
        string image
        int port
        string resources
        string livenessProbe
        string readinessProbe
    }
```

**Key relationships:**
- **Namespace** owns all resources within it — deleting a namespace cascades to everything
- **Deployment** manages pods via label selectors — changing the selector orphans existing pods
- **PVC** persists independently of the Deployment — survives pod restarts and node failures
- **Service** bridges ephemeral pod IPs to a stable DNS name via label selector
- **ExternalName Service** bridges the `hkask` namespace to `hkask-conduit` so the Ingress can route `/_matrix`
- **PDB** prevents voluntary eviction of the sole pod — `maxUnavailable: 0`

For the architecture overview, see `docs/diagrams/flowchart-deployment-architecture.md`.
For the startup sequence, see `docs/diagrams/flowchart-pod-startup.md`.
