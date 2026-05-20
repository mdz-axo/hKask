---
title: "hKask Macaroon Issuer Guide"
audience: [hKask developers, Russell developers]
last_updated: 2026-05-20
togaf_phase: "D"
version: "1.0.0"
status: "Active"
---

<!-- TOGAF_DOMAIN: Application -->
<!-- VERSION: 1.0.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-20 -->

# hKask Macaroon Issuer Guide

This guide describes how hKask issues macaroons to Russell ACP agents for skill registration and Okapi inference access.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    hKask (Macaroon Issuer)                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Root Keys:                                                  │   │
│  │  - rk_hkask_skill_registry (skill registration)              │   │
│  │  - rk_hkask_mcp (MCP tool access)                            │   │
│  │  - rk_hkask_okapi_discharge (Okapi third-party discharge)    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Macaroon Issuer Service:                                           │
│  - Issues macaroons to Russell on skill registration               │
│  - Provides discharge service for third-party caveats              │
│  - Tracks per-skill quota and audit logs                           │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             │ Russell registers skill
                             │ hKask issues macaroon with:
                             │ - iid: russell-prod-1
                             │ - skill: evolution-watcher
                             │ - before: 2026-05-21T00:00:00Z
                             │ - third_party: okapi-access
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Russell (Macaroon Holder + Attenuator)                 │
│  Stores macaroon in secure keychain                                 │
│  Attenuates per skill invocation                                    │
│  Requests discharge for Okapi access                                │
└─────────────────────────────────────────────────────────────────────┘
```

## Root Key Configuration

hKask maintains separate root keys for each capability:

```yaml
# ~/.config/hkask/macaroon.yaml
macaroon:
  issuer:
    enabled: true
    
    root_keys:
      rk_hkask_skill_registry:
        key: <base64-encoded-key>
        purpose: "Skill registration macaroons"
        rotation_days: 90
      
      rk_hkask_mcp:
        key: <base64-encoded-key>
        purpose: "MCP tool access macaroons"
        rotation_days: 90
      
      rk_hkask_okapi_discharge:
        key: <base64-encoded-key>
        purpose: "Okapi third-party discharge"
        rotation_days: 90
        matches_okapi_key: rk_okapi_discharge
```

### Generate Root Keys

```bash
# Generate keys for hKask
hkask macaroon key generate \
  --name rk_hkask_skill_registry \
  --purpose "Skill registration macaroons"

hkask macaroon key generate \
  --name rk_hkask_mcp \
  --purpose "MCP tool access macaroons"

hkask macaroon key generate \
  --name rk_hkask_okapi_discharge \
  --purpose "Okapi third-party discharge"
```

### Sync with Okapi

The `rk_hkask_okapi_discharge` key must match Okapi's `rk_okapi_discharge`:

```bash
# Export Okapi discharge key
okapi macaroon key export --name rk_okapi_discharge > /tmp/discharge_key.json

# Import to hKask
hkask macaroon key import \
  --name rk_hkask_okapi_discharge \
  --from /tmp/discharge_key.json
```

## Skill Registration Flow

### 1. Russell Registers Skill

```json
POST /api/v1/agents/russell/skills/register
{
  "agent_id": "russell-prod-1",
  "skill": "evolution-watcher",
  "version": "1.0.0",
  "endpoints_required": [
    "/api/evolution/scan",
    "/api/evolution/propose",
    "/api/evolution/execute"
  ],
  "models_required": ["qwen3:8b", "qwen3:70b"],
  "okapi_access_required": true
}
```

### 2. hKask Issues Primary Macaroon

```go
// hKask macaroon issuer service
func (s *MacaroonIssuer) IssueSkillMacaroon(req SkillRegistrationRequest) (*Macaroon, error) {
    rootKey := s.RootKeys["rk_hkask_skill_registry"]
    
    m, err := macaroon.New(rootKey, "rk_hkask_skill_registry", req.AgentID)
    if err != nil {
        return nil, err
    }
    
    // Identity caveats
    m.AddFirstPartyCaveat(fmt.Sprintf("iid:%s", req.AgentID))
    m.AddFirstPartyCaveat(fmt.Sprintf("skill:%s", req.Skill))
    m.AddFirstPartyCaveat(fmt.Sprintf("jti:%s", uuid.New()))
    
    // Temporal caveats
    expiry := time.Now().Add(24 * time.Hour)
    m.AddFirstPartyCaveat(fmt.Sprintf("before:%s", expiry.Format(time.RFC3339)))
    
    // Capability caveats
    m.AddFirstPartyCaveat("activity:skill:invoke")
    for _, ep := range req.EndpointsRequired {
        m.AddFirstPartyCaveat(fmt.Sprintf("endpoint:%s", ep))
    }
    
    // Resource caveats
    for _, model := range req.ModelsRequired {
        m.AddFirstPartyCaveat(fmt.Sprintf("model:%s", model))
    }
    
    // Quota caveats
    m.AddFirstPartyCaveat("quota:1000000-tokens/day")
    m.AddFirstPartyCaveat("rpm:100")
    
    // Third-party caveat for Okapi access
    if req.OkapiAccessRequired {
        m.AddThirdPartyCaveat("okapi-access", "rk_hkask_okapi_discharge")
    }
    
    return m, nil
}
```

### 3. hKask Responds to Russell

```json
{
  "status": "registered",
  "skill": "evolution-watcher",
  "macaroon": "<base64-encoded-primary-macaroon>",
  "discharge_endpoint": "http://127.0.0.1:8080/mcp/v1/discharge",
  "expires_at": "2026-05-21T00:00:00Z"
}
```

## Third-Party Discharge Flow

### 1. Russell Requests Discharge

```json
POST /api/v1/mcp/discharge
{
  "primary_macaroon": "<base64-encoded-primary>",
  "location": "okapi-access",
  "agent_id": "russell-prod-1",
  "skill": "evolution-watcher"
}
```

### 2. hKask Verifies Russell Identity

```go
func (s *DischargeService) IssueDischarge(req DischargeRequest) (*Macaroon, error) {
    // Verify primary macaroon
    primary, err := macaroon.Deserialize(req.PrimaryMacaroon)
    if err != nil {
        return nil, fmt.Errorf("invalid primary macaroon: %w", err)
    }
    
    caveats := primary.ParseCaveats()
    
    // Verify agent identity matches
    if caveats.IID != req.AgentID {
        return nil, fmt.Errorf("agent ID mismatch")
    }
    
    // Verify skill is registered
    if !s.SkillRegistry.IsRegistered(req.AgentID, req.Skill) {
        return nil, fmt.Errorf("skill not registered")
    }
    
    // Verify third_party caveat exists
    if caveats.ThirdParty != "okapi-access" {
        return nil, fmt.Errorf("third_party caveat not found")
    }
    
    // Issue discharge macaroon
    rootKey := s.RootKeys["rk_hkask_okapi_discharge"]
    discharge, err := macaroon.New(rootKey, "rk_hkask_okapi_discharge", fmt.Sprintf("discharge-%s", req.AgentID))
    if err != nil {
        return nil, err
    }
    
    discharge.AddFirstPartyCaveat("okapi_access:true")
    discharge.AddFirstPartyCaveat(fmt.Sprintf("models:%s", strings.Join(caveats.Models, ",")))
    discharge.AddFirstPartyCaveat(fmt.Sprintf("before:%s", caveats.Before.Format(time.RFC3339)))
    discharge.AddFirstPartyCaveat(fmt.Sprintf("iid:%s", req.AgentID))
    
    return discharge, nil
}
```

### 3. hKask Returns Discharge Macaroon

```json
{
  "status": "success",
  "discharge_macaroon": "<base64-encoded-discharge>"
}
```

## MCP Tool Authorization

When Russell invokes an MCP tool, hKask verifies the macaroon:

```go
func (s *MCPServer) AuthorizeRequest(r *http.Request) error {
    authHeader := r.Header.Get("Authorization")
    if authHeader == "" {
        return fmt.Errorf("authentication required")
    }
    
    parts := strings.SplitN(authHeader, " ", 2)
    if len(parts) != 2 || parts[0] != "Bearer" {
        return fmt.Errorf("invalid authorization header")
    }
    
    macaroon, err := macaroon.Deserialize(parts[1])
    if err != nil {
        return fmt.Errorf("invalid macaroon: %w", err)
    }
    
    // Verify HMAC chain
    if err := macaroon.Verify(s.RootKeys); err != nil {
        return fmt.Errorf("macaroon verification failed: %w", err)
    }
    
    caveats := macaroon.ParseCaveats()
    
    // Check temporal caveats
    if time.Now().After(caveats.Before) {
        return fmt.Errorf("macaroon expired")
    }
    
    // Check endpoint caveat
    if !caveats.Endpoints.Contains(r.URL.Path) {
        return fmt.Errorf("endpoint not allowed")
    }
    
    // Check skill caveat
    if caveats.Skill == "" {
        return fmt.Errorf("skill caveat required")
    }
    
    // Add to context for audit logging
    ctx := r.Context()
    ctx = context.WithValue(ctx, "client_id", caveats.IID)
    ctx = context.WithValue(ctx, "skill", caveats.Skill)
    *r = *r.WithContext(ctx)
    
    return nil
}
```

## Audit Logging

hKask logs all macaroon operations:

```json
{
  "timestamp": "2026-05-20T10:30:00Z",
  "event": "macaroon_issued",
  "agent_id": "russell-prod-1",
  "skill": "evolution-watcher",
  "macaroon_id": "mac-abc123",
  "caveats": {
    "iid": "russell-prod-1",
    "skill": "evolution-watcher",
    "endpoints": ["/api/evolution/scan"],
    "models": ["qwen3:8b"],
    "before": "2026-05-21T00:00:00Z"
  }
}
```

```json
{
  "timestamp": "2026-05-20T10:31:00Z",
  "event": "discharge_issued",
  "agent_id": "russell-prod-1",
  "skill": "evolution-watcher",
  "discharge_id": "discharge-abc123",
  "okapi_access": true
}
```

```json
{
  "timestamp": "2026-05-20T10:32:00Z",
  "event": "mcp_tool_invoked",
  "agent_id": "russell-prod-1",
  "skill": "evolution-watcher",
  "tool": "inference/generate",
  "model": "qwen3:8b",
  "tokens_generated": 128
}
```

## Quota Enforcement

hKask tracks per-skill, per-agent quotas:

```go
type QuotaManager struct {
    mu sync.Mutex
    quotas map[string]*AgentQuota  // key: agent_id:skill
}

type AgentQuota struct {
    AgentID           string
    Skill             string
    TokensUsed        int64
    TokensLimit       int64
    RequestsUsed      int64
    RequestsLimit     int64
    ResetAt           time.Time
}

func (q *QuotaManager) Check(agentID, skill string, tokens int64) bool {
    q.mu.Lock()
    defer q.mu.Unlock()
    
    key := fmt.Sprintf("%s:%s", agentID, skill)
    quota, ok := q.quotas[key]
    if !ok {
        return true  // No quota configured
    }
    
    if time.Now().After(quota.ResetAt) {
        quota.TokensUsed = 0
        quota.RequestsUsed = 0
        quota.ResetAt = time.Now().Add(24 * time.Hour)
    }
    
    if quota.TokensUsed+tokens > quota.TokensLimit {
        return false
    }
    
    quota.TokensUsed += tokens
    quota.RequestsUsed++
    return true
}
```

## Metrics

hKask exposes macaroon-related Prometheus metrics:

```
# Macaroon issuances total
hkask_macaroon_issued_total{agent_id,skill}

# Discharge issuances total
hkask_discharge_issued_total{agent_id,skill}

# MCP tool invocations total
hkask_mcp_invocations_total{agent_id,skill,tool}

# Token usage per agent/skill
hkask_tokens_generated_total{agent_id,skill}

# Quota remaining
hkask_quota_remaining{agent_id,skill,type="tokens"}
hkask_quota_remaining{agent_id,skill,type="requests"}

# Authorization failures
hkask_auth_failures_total{agent_id,reason}
```

## Configuration Reference

```yaml
# ~/.config/hkask/config.yaml
macaroon:
  issuer:
    enabled: true
    
    root_keys:
      rk_hkask_skill_registry:
        key: <base64-key>
        purpose: "Skill registration"
        rotation_days: 90
      
      rk_hkask_mcp:
        key: <base64-key>
        purpose: "MCP tool access"
        rotation_days: 90
      
      rk_hkask_okapi_discharge:
        key: <base64-key>
        purpose: "Okapi discharge"
        rotation_days: 90
        matches_okapi_key: rk_okapi_discharge
    
    default_caveats:
      before: 24h
      quota: 1000000-tokens/day
      rpm: 100
    
    skills:
      evolution-watcher:
        endpoints:
          - /api/evolution/scan
          - /api/evolution/propose
          - /api/evolution/execute
        models:
          - qwen3:8b
          - qwen3:70b
        quota:
          tokens_per_day: 5000000
          requests_per_minute: 50
      
      rdf-embedding:
        endpoints:
          - /api/embed/rdf
        models:
          - qwen3-embedding:0.6b
        quota:
          tokens_per_day: 10000000
          requests_per_minute: 20
      
      perf-benchmark:
        endpoints:
          - /api/generate
          - /api/chat
        models:
          - "*"
        quota:
          tokens_per_day: 100000000
          requests_per_minute: 100
    
    audit:
      enabled: true
      log_file: /var/log/hkask/macaroon_audit.log
      retention_days: 90
```

## References

- `fork-docs/AUTH_SPEC.md` — Okapi macaroon authentication
- `fork-docs/MACAROON_SPEC.md` — Macaroon caveat vocabulary
- `fork-docs/MACAROON_DEPLOYMENT.md` — Deployment guide
- `server/macaroon_auth.go` — Okapi macaroon middleware
- `cmd/cmd_macaroon.go` — Macaroon CLI commands

---

*hKask v0.22.0 — Macaroon issuer for Russell ACP agents*
