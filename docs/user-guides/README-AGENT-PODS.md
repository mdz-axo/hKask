---
title: "Agent Pod Documentation Index"
audience: [developers, agent designers]
last_updated: 2026-05-24
togaf_phase: "G"
version: "0.21.0"
status: "Active"
domain: "Application"
---

# Agent Pod Documentation Index

---

## Overview

This index provides quick access to all documentation for creating and managing agent pods in hKask[^hewitt1973].

---

## User Guides

### Core Documentation

| Document | Purpose | Audience |
|----------|---------|----------|
| **[AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md)** | Complete step-by-step guide for creating agent pods | All users |
| **[AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md](./AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md)** | Requirements discovery questionnaire | Architects, developers |
| **[COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md)** | Catalog of common agent patterns and templates | Developers, architects |

### Quick Start

**New to hKask?** Start here[^wiegers2013]:

1. Read [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md) — Sections 1-3
2. Complete [AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md](./AGENT-POD-REQUIREMENTS-QUESTIONNAIRE.md) — Sections 1-8
3. Run template generator: `./scripts/generate-agent-pod.sh`
4. Review [COMMON-AGENT-PATTERNS.md](./COMMON-AGENT-PATTERNS.md) for examples
5. Register and activate your agent pod

---

## Scripts

Automation scripts support continuous delivery workflows[^humble2010]:

| Script | Purpose | Usage |
|--------|---------|-------|
| `scripts/generate-agent-pod.sh` | Interactive agent pod crate generator | `./scripts/generate-agent-pod.sh` |

---

## Architecture Documentation

Related architecture documents[^bass2021]:

| Document | Purpose |
|----------|---------|
| [hKask-architecture-master.md](../architecture/hKask-architecture-master.md) | Master architecture specification |
| [hKask-erd.md](../architecture/hKask-erd.md) | Entity relationship diagrams |
| [registry-templating-prompt-v2.md](../architecture/registry-templating-prompt-v2.md) | Registry and templating design |
| [security-architecture.md](../architecture/security-architecture.md) | OCAP security model |
| [AGENT_POD_IMPLEMENTATION.md](../architecture/AGENT_POD_IMPLEMENTATION.md) | Agent pod implementation details |

---

## API Reference

API endpoints follow REST conventions[^fielding2000].

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

Templates use YAML 1.2[^yaml12] and Jinja2[^jinja2] formats.

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

Patterns follow established software design pattern conventions[^gamma1994]:

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

Diagnostic approaches follow security testing methodology[^owasp_testing]:

| Problem | Solution |
|---------|----------|
| Pod creation fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-creation-fails) |
| Pod registration fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-registration-fails) |
| Pod activation fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#pod-activation-fails) |
| CNS span emission fails | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#cns-span-emission-fails) |
| Visibility/access errors | [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md#visibilityaccess-errors) |

---

## Checklist: Agent Pod Creation

Architecture readiness checklists ensure deployment quality[^bass2021]:

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

Terminology follows multiagent systems conventions[^wooldridge2009]:

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
- [hKask Curator Persona](../architecture/hKask-Curator-persona.md)

### External References
- [ACP Runtime Documentation](https://github.com/acp-runtime/acp-runtime)
- [MCP Protocol Specification](https://modelcontextprotocol.io/)[^mcp_spec]
- [OCAP Security Model](https://www.erights.org/ocap/)

---

## Support

For questions or issues[^fogel2005]:
1. Check [Troubleshooting](#troubleshooting) section
2. Review [AGENT-POD-CREATION-GUIDE.md](./AGENT-POD-CREATION-GUIDE.md)
3. Consult architecture documentation
4. Contact hKask Administrator

[^hewitt1973]: Hewitt, C., Bishop, P., & Steiger, R. (1973). A universal modular ACTOR formalism for artificial intelligence. In *Proceedings of the 3rd International Joint Conference on Artificial Intelligence (IJCAI)* (pp. 235-245). https://dl.acm.org/doi/10.5555/1624775.1624804
[^wiegers2013]: Wiegers, K. E., & Beatty, J. (2013). *Software requirements* (3rd ed.). Microsoft Press.
[^humble2010]: Humble, J., & Farley, D. (2010). *Continuous delivery: Reliable software releases through build, test, and deployment automation*. Addison-Wesley.
[^bass2021]: Bass, L., Clements, P., & Kazman, R. (2021). *Software architecture in practice* (4th ed.). Addison-Wesley.
[^fielding2000]: Fielding, R. T. (2000). *Architectural styles and the design of network-based software architectures* [Doctoral dissertation, University of California, Irvine]. https://www.ics.uci.edu/~fielding/pubs/dissertation/top.htm
[^yaml12]: Ben-Kiki, O., Evans, C., & döt Net, I. (2009). *YAML ain't markup language (YAML) version 1.2* (3rd ed.). https://yaml.org/spec/1.2/spec.html
[^jinja2]: Ronacher, A. (2024). *Jinja2 documentation*. Pallets Projects. https://jinja.palletsprojects.com/
[^gamma1994]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design patterns: Elements of reusable object-oriented software*. Addison-Wesley.
[^owasp_testing]: OWASP Foundation. (2024). *OWASP web security testing guide, v4.2*. https://owasp.org/www-project-web-security-testing-guide/
[^wooldridge2009]: Wooldridge, M. (2009). *An introduction to multiagent systems* (2nd ed.). Wiley.
[^mcp_spec]: Anthropic. (2024). *Model Context Protocol specification*. https://modelcontextprotocol.io/
[^fogel2005]: Fogel, K. (2005). *Producing open source software: How to run a successful free software project*. O'Reilly. https://producingoss.com/

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
