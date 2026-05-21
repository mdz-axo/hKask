# Agent Pod Documentation Index

**Version:** v0.21.0  
**Last Updated:** 2026-05-20

---

## Overview

This index provides quick access to all documentation for creating and managing agent pods in hKask.

---

## User Guides

### Core Documentation

| Document | Purpose | Audience |
|----------|---------|----------|
| **[AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md)** | Complete step-by-step guide for creating agent pods | All users |
| **[AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md](./AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md)** | Requirements discovery questionnaire | Architects, developers |
| **[COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md)** | Catalog of common agent patterns and templates | Developers, architects |

### Quick Start

**New to hKask?** Start here:

1. Read [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md) — Sections 1-3
2. Complete [AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md](./AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md) — Sections 1-8
3. Run template generator: `./scripts/generate-agent-pod.sh`
4. Review [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md) for examples
5. Register and activate your agent pod

---

## Scripts

| Script | Purpose | Usage |
|--------|---------|-------|
| `scripts/generate-agent-pod.sh` | Interactive agent pod crate generator | `./scripts/generate-agent-pod.sh` |

---

## Architecture Documentation

Related architecture documents:

| Document | Purpose |
|----------|---------|
| [hKask-architecture-master.md](../architecture/hKask-architecture-master.md) | Master architecture specification |
| [hKask-erd.md](../architecture/hKask-erd.md) | Entity relationship diagrams |
| [registry-templating-prompt-v2.md](../architecture/registry-templating-prompt-v2.md) | Registry and templating design |
| [security-architecture.md](../architecture/security-architecture.md) | OCAP security model |
| [AGENT_POD_IMPLEMENTATION.md](../architecture/AGENT_POD_IMPLEMENTATION.md) | Agent pod implementation details |

---

## API Reference

### Pod Management Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/pods` | GET | List all pods |
| `/api/pods` | POST | Create new pod |
| `/api/pods/:id` | GET | Get pod status |
| `/api/pods/:id/activate` | POST | Activate pod |
| `/api/pods/:id/deactivate` | POST | Deactivate pod |

### CLI Commands

| Command | Description |
|---------|-------------|
| `kask pod create` | Create new pod |
| `kask pod list` | List all pods |
| `kask pod status` | Get pod status |
| `kask pod activate` | Activate pod |
| `kask pod deactivate` | Deactivate pod |

---

## File Templates

### Agent Persona

**Location:** `agent_persona.yaml`

Required fields:
- `agent.name` — Agent name (kebab-case)
- `agent.type` — Bot or Replicant
- `agent.binding_contract` — Must be `true`
- `charter.description` — Purpose statement
- `capabilities` — List of tool capabilities
- `rights` — Read/execute/write rights
- `responsibilities` — Agent responsibilities
- `visibility` — Default and episodic visibility
- `process_manifest` — Path to dispatch manifest
- `readiness_probe` — Health check configuration

### Dispatch Manifest

**Location:** `dispatch_manifest.yaml`

Required fields:
- `manifest.name` — Manifest name
- `manifest.version` — Version string
- `matroshka.max_depth` — Max recursion depth (≤7)
- `steps` — Array of workflow steps
- `cns.spans` — CNS span emission list

### Template Files

**Location:** `templates/`

Required structure:
```
templates/
├── selectors/
│   └── selector.j2
├── prompts/
│   └── prompt_*.j2
├── processes/
│   └── process_*.yaml
└── cognitions/
    └── cognition_*.j2
```

---

## Common Patterns by Use Case

### I need a bot that...

| Use Case | Pattern | Reference |
|----------|---------|-----------|
| Performs domain-specific operations | Specialist Bot | [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md#pattern-1-specialist-bot) |
| Monitors system health | Curator Bot | [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md#pattern-2-curator-bot) |
| Routes template requests | Dispatch Bot | [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md#pattern-3-dispatch-bot) |
| Assists human users | Replicant Assistant | [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md#pattern-4-replicant-assistant) |
| Connects external workspace | Bridge Agent | [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md#pattern-5-bridge-agent) |

### I need to...

| Task | Guide Section |
|------|---------------|
| Create a new agent pod | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#step-1-requirements-discovery) |
| Define agent capabilities | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#12-capabilities-required) |
| Configure visibility | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#step-8-configure-visibility) |
| Create dispatch workflow | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#step-3-create-dispatch-manifest) |
| Register with ACP runtime | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#step-6-register-with-acp-runtime) |
| Activate agent pod | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#step-7-activate-pod) |

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Pod creation fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-creation-fails) |
| Pod registration fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-registration-fails) |
| Pod activation fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-activation-fails) |
| CNS span emission fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#cns-span-emission-fails) |
| Visibility/access errors | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#visibilityaccess-errors) |

---

## Checklist: Agent Pod Creation

### Before Creation
- [ ] Complete requirements questionnaire
- [ ] Select agent pattern
- [ ] Define capabilities and rights
- [ ] Configure visibility settings
- [ ] Identify dependencies

### Crate Structure
- [ ] `Cargo.toml` created
- [ ] `agent_persona.yaml` created
- [ ] `dispatch_manifest.yaml` created
- [ ] `hlexicon.yaml` created (optional)
- [ ] Templates directory structure created
- [ ] Selector templates created
- [ ] Prompt templates created
- [ ] Process templates created (if needed)
- [ ] Cognition templates created (if needed)

### Registration
- [ ] Crate built successfully
- [ ] Pod created via CLI/API
- [ ] Capability token received
- [ ] ACP registration confirmed

### Activation
- [ ] Pod activated
- [ ] MCP tool access granted
- [ ] CNS span emission verified
- [ ] Readiness probe passed

### Post-Activation
- [ ] Joined ensemble session (if applicable)
- [ ] Monitoring CNS spans
- [ ] Producing memory artifacts
- [ ] Coordinating with other bots

---

## Glossary

| Term | Definition |
|------|------------|
| **ACP** | Agent Communication Protocol |
| **A2A** | Agent-to-Agent communication |
| **Bot** | Machine-to-machine agent (process execution) |
| **Replicant** | Human-to-agent assistant |
| **OCAP** | Object-Capability security model |
| **CNS** | Cybernetic Nervous System (monitoring) |
| **MCP** | Model Context Protocol (tool access) |
| **Matroshka** | Recursion depth limiting mechanism |
| **WebID** | Web identifier for agents |
| **Macaroon** | Capability token with caveats |

---

## Additional Resources

### Related Documentation
- [CLI/API Symmetry Audit](../architecture/cli-api-symmetry-audit.md)
- [Bot System Setup](../architecture/BOT_SYSTEM_SETUP.md)
- [hKask Curator Persona](../architecture/hKask-Curator-persona.md)

### External References
- [ACP Runtime Documentation](https://github.com/acp-runtime/acp-runtime)
- [MCP Protocol Specification](https://modelcontextprotocol.io/)
- [OCAP Security Model](https://www.erights.org/ocap/)

---

## Support

For questions or issues:
1. Check [Troubleshooting](#troubleshooting) section
2. Review [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md)
3. Consult architecture documentation
4. Contact hKask Administrator

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
