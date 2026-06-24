# hkask-federation — Pod Federation

Multi-pod federation layer: cross-pod communication, federation links, agent discovery across pod boundaries, and federation protocol handling.

**Version:** v0.30.0 | **Crate:** `hkask-federation`

## Purpose

Enables hKask pods to federate — discovering agents in remote pods, routing A2A messages across pod boundaries, and maintaining federation link state.

## Key Types

- `FederationDispatch` — trait for cross-pod message routing
- `FederationLinkManager` — active federation link state
- `FederationLink` — individual pod-to-pod connection

## Dependencies

- `hkask-types` — WebID, CNS spans
- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-cns` — CNS span emission
- `hkask-communication` — Agent registry, Matrix transport
