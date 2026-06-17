#!/usr/bin/env python3
"""
Add 'description' fields to endpoints in openapi.json that lack them.
This script is ad-hoc (to be deleted after use) per AGENTS.md Python policy.
"""

import json
import sys

DESCRIPTIONS = {
    # -- bots --
    ("/api/bots/{id}/capabilities", "get"): (
        "List all capabilities currently held by a specific bot. "
        "Requires the bot's WebID as a path parameter. "
        "Returns a JSON array of capability string identifiers. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/bots/{id}/grant", "post"): (
        "Grant a new capability to a specific bot. "
        "Requires the bot's WebID as a path parameter and a GrantCapabilityRequest body "
        "specifying the capability to grant. "
        "Returns a 200 confirmation on success, 400 for invalid requests. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- cns --
    ("/api/cns/health", "get"): (
        "Get CNS health status for the entire system. "
        "Returns CnsHealthResponse including overall health boolean, "
        "critical alert count, warning count, and total variety deficit. "
        "Use this endpoint to monitor system homeostasis. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/cns/variety", "get"): (
        "Get CNS variety counter values across all monitored domains. "
        "Returns CnsVarietyResponse with per-domain counters, "
        "domain list, and total deficit. "
        "Variety counters track information diversity per Ashby's Law of Requisite Variety. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- episodic --
    ("/api/episodic/usage", "get"): (
        "Get episodic memory usage statistics for the authenticated caller. "
        "Returns EpisodicUsageResponse with current memory count and storage budget. "
        "Use this to monitor memory consumption before hitting storage limits. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- goals --
    ("/api/goals", "get"): (
        "List all MDS goals for the authenticated agent, optionally filtered by lifecycle state. "
        "Accepts an optional `state` query parameter to filter by goal state. "
        "Returns GoalListResponse with an array of goals (id, text, state, visibility). "
        "Requires DelegationToken auth (P4 OCAP); 403 if authority denied."
    ),
    ("/api/goals", "post"): (
        "Capture a new MDS goal for the authenticated agent. "
        "Requires a CreateGoalRequest body with goal text and visibility. "
        "Returns GoalResponse with the created goal's id, state, text, and visibility. "
        "Requires DelegationToken auth (P4 OCAP); 403 if authority denied."
    ),
    ("/api/goals/{id}/state", "post"): (
        "Transition a goal to a new lifecycle state. Only legal state transitions are accepted. "
        "Requires the goal ID as a path parameter and a SetGoalStateRequest body "
        "with the target state. "
        "Returns 200 on success, 400 for illegal transitions, 404 if goal not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- mcp --
    ("/api/mcp/servers", "get"): (
        "List all registered MCP servers currently connected to the hKask runtime. "
        "Returns a JSON array of server name strings. "
        "Use this to discover available tool providers before invoking tools. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- sovereignty --
    ("/api/sovereignty/access/check", "get"): (
        "Check whether the authenticated agent has access to a specific data category "
        "through the P4 (OCAP) membrane. "
        "Requires a `category` query parameter specifying the data category to check. "
        "Returns AccessCheckResponse with classification (PUBLIC/SHARED/SOVEREIGN) "
        "and the access gate required. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/sovereignty/consent/grant", "post"): (
        "Grant explicit sovereign consent for a data category, enabling data sharing "
        "under P2 (Affirmative Consent). "
        "Requires a SovereigntyConsentRequest body specifying the category. "
        "Returns SovereigntyConsentResponse with updated consent state "
        "and granted categories list. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/sovereignty/consent/revoke", "post"): (
        "Revoke all explicit sovereign consent — only public data remains accessible. "
        "This is a global revoke (P2 Affirmative Consent withdrawal). "
        "Returns SovereigntyConsentResponse confirming revocation. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/sovereignty/status", "get"): (
        "Get sovereignty status for the authenticated agent: consent state, "
        "data category classifications (public, shared, sovereign), "
        "and explicitly granted sharing categories. "
        "Returns SovereigntyStatusResponse. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- specs --
    ("/api/specs", "get"): (
        "List all specifications in the system. "
        "Returns a JSON array of SpecListResponse objects "
        "with spec_id, name, category, and completeness flag. "
        "Use this to discover available specifications before querying details. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/specs/capture", "post"): (
        "Capture a new specification (MDS §3: spec/goal/capture). "
        "Requires a SpecCaptureRequestDto body with description and optional context. "
        "Returns the captured specification on success. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/specs/coherence", "get"): (
        "Get specification collection coherence score (MDS §3: spec/graph/coherence). "
        "Analyzes cross-spec consistency — detects contradictions, violations, "
        "and provides improvement suggestions. "
        "Returns SpecCoherenceResponse with coherence_score, violations, and suggestions. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/specs/{spec_id}", "get"): (
        "Get a single specification by its unique ID. "
        "Requires the spec_id as a path parameter. "
        "Returns SpecDetailResponse with the specification's name, category, "
        "domain anchor, and full requirements list. "
        "Returns 404 if the spec is not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/specs/{spec_id}/writing-quality", "get"): (
        "Get writing quality assessment for a specification (MDS §3: spec/require/writing-quality). "
        "Requires the spec_id as a path parameter. "
        "Returns SpecWritingQualityResponse with dimensions_passing count "
        "and meets_publication_standard flag. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- templates --
    ("/api/templates", "get"): (
        "List all registered templates across the system. "
        "Returns a JSON array of TemplateResponse objects "
        "with template id, name, description, type, source path, and lexicon terms. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/templates/{id}", "get"): (
        "Get a single template by its unique ID. "
        "Requires the template ID as a path parameter. "
        "Returns TemplateResponse with full template metadata including name, "
        "description, template_type, source_path, and lexicon_terms. "
        "Returns 404 if the template is not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- acp --
    ("/api/v1/acp/agents", "get"): (
        "List all registered ACP agents with their WebID, agent type, "
        "capabilities, active status, and registration timestamp. "
        "Returns AgentListResponse. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/acp/agents/{agent_id}", "delete"): (
        "Unregister and remove an ACP agent by its agent ID. "
        "Requires the agent_id as a path parameter. "
        "Returns 200 on success, 404 if agent not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- backup --
    ("/api/v1/backup/config", "get"): (
        "Get current backup configuration including auto-snapshot setting, "
        "retention policy, tracked artifact types, and verify-after-snapshot flag. "
        "Returns BackupConfigResponse. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/config", "put"): (
        "Update backup configuration. "
        "Requires an UpdateConfigRequest body with optional fields for auto_snapshot, "
        "retention, tracked_types, and verify_after_snapshot. "
        "Returns the updated BackupConfigResponse. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/list", "get"): (
        "List backup snapshots with optional type and limit filters. "
        "Accepts optional `type` (filter by artifact type) and `limit` "
        "(max snapshots to return) query parameters. "
        "Returns ListResponse with an array of snapshot metadata. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/prune", "post"): (
        "Prune expired backup snapshots according to the configured retention policy. "
        "Requires a PruneRequest body with an optional `dry_run` flag "
        "to preview removals without deleting. "
        "Returns PruneResponse with counts of evaluated, retained, and removed snapshots. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/restore", "post"): (
        "Restore artifacts from a specific backup snapshot. "
        "Requires a RestoreRequest body with commit_hash and scope "
        "(all artifacts, by type and IDs, or by specific type). "
        "Returns RestoreResponse with the list of restored artifact IDs. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/snapshot", "post"): (
        "Create a new backup snapshot of tracked artifacts. "
        "Requires a SnapshotRequest body specifying the scope "
        "(all tracked types or specific artifact types/IDs). "
        "Returns SnapshotResponse with artifact_count, commit hashes, "
        "timestamp, and trigger info. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/backup/verify", "post"): (
        "Verify integrity of all backup snapshots by checking git object hashes. "
        "No request body required. "
        "Returns VerifyResponse with per-repository verification reports "
        "including ok flag, verified_blobs, total_blobs, and corrupt_hashes. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- bundles --
    ("/api/v1/bundles", "get"): (
        "List all skill bundles. "
        "Returns BundleListResponse with bundle summaries including id, name, "
        "description, version, skill_count, and visibility. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/bundles/compose", "post"): (
        "Compose a new skill bundle from specified skills. "
        "Requires a ComposeBundleRequest body with name, skills list, and visibility. "
        "Returns ComposeBundleResponse with the composed manifest, message, and warnings. "
        "Returns 400 for invalid skill references. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/bundles/{id}", "get"): (
        "Get a specific skill bundle by its ID. "
        "Requires the bundle ID as a path parameter. "
        "Returns the full bundle manifest as JSON. "
        "Returns 404 if the bundle is not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/bundles/{id}/apply", "post"): (
        "Apply a skill bundle to the current session, activating all its skills. "
        "Requires the bundle ID as a path parameter. "
        "Returns ApplyBundleResponse with bundle_id, name, skill_count, and status. "
        "Returns 404 if the bundle is not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/bundles/{id}/deactivate", "delete"): (
        "Deactivate an active skill bundle, unloading all its skills from the session. "
        "Requires the bundle ID as a path parameter. "
        "Returns DeactivateBundleResponse confirming the new status. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/bundles/{id}/evolve", "post"): (
        "Evolve a bundle by re-composing it when its constituent skills have changed. "
        "Requires the bundle ID as a path parameter. "
        "Returns EvolveBundleResponse with the evolved manifest, changes list, and message. "
        "Returns 404 if the bundle is not found. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- curator --
    ("/api/v1/curator/escalations", "get"): (
        "List all pending curator escalations requiring human attention. "
        "Returns ListEscalationsResponse with escalation entries including "
        "id, bot_id, status, created_at, error_context, and retry_count. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/curator/escalations/{id}/dismiss", "post"): (
        "Dismiss a curator escalation as non-actionable. "
        "Requires the escalation ID as a path parameter and a DismissEscalationRequest body "
        "recording who dismissed it. "
        "Returns DismissEscalationResponse with id and updated status. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/curator/escalations/{id}/resolve", "post"): (
        "Resolve a curator escalation, marking it as handled. "
        "Requires the escalation ID as a path parameter and a ResolveEscalationRequest body "
        "recording who resolved it. "
        "Returns ResolveEscalationResponse with id and updated status. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/v1/curator/metacognition", "get"): (
        "Get Curator metacognition status including escalation statistics "
        "and per-bot health reports. "
        "Returns MetacognitionStatusResponse with bot_reports and escalation_stats. "
        "Use this to monitor bot health and system self-awareness. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- git --
    ("/api/v1/git/resolve/{sha}", "get"): (
        "Resolve a git reference (branch, tag, or commit prefix) to a full commit SHA. "
        "Uses GitCASPort for resolution. "
        "Requires the reference as a path parameter. "
        "Returns ResolveShaResponse with the resolved SHA. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    # -- wallet --
    ("/api/wallet/balance", "get"): (
        "Get the current wallet balance. "
        "Returns WalletBalanceResponse with rJoules balance, "
        "USDC equivalent, gas equivalent, and wallet ID. "
        "Returns 503 if the wallet service is not configured. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/deposit-address", "get"): (
        "Get a deposit address for receiving USDC. "
        "Accepts optional query parameters: `chain` (blockchain), `private` (privacy mode), "
        "and `wallet_id` (defaults to system wallet). "
        "Returns DepositAddressResponse with address, chain, and privacy mode. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/deposit-reference", "post"): (
        "Generate a one-time deposit reference for shielded deposits. "
        "Requires a DepositReferenceRequest body with chain and optional wallet_id. "
        "Returns DepositReferenceResponse with reference string, chain, and expiry. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/fee", "get"): (
        "Estimate current network withdrawal fee using the configured price feed. "
        "Accepts an optional `chain` query parameter to specify the blockchain. "
        "Returns WithdrawalFeeEstimateResponse with fee in rJoules, "
        "native units, and USDC equivalent. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/keys", "get"): (
        "List all active API keys with their status, spending limits, and expiry. "
        "Returns ApiKeyListResponse with key entries including key_id, "
        "limit_rj, spent_rj, status, and expires_at. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/keys", "post"): (
        "Create a new API key with spending limits and expiry. "
        "Requires a CreateKeyRequest body specifying chain, limit_rj, "
        "expiry_days, private mode, and wallet_id. "
        "Returns ApiKeyCreatedResponse with the key_id, private_key_hex, "
        "spending_limit_rj, and expires_at. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/keys/{key_id}", "delete"): (
        "Revoke an API key, immediately disabling it. "
        "Requires the key_id as a path parameter. "
        "Returns ApiKeyRevokedResponse confirming revocation. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/transactions", "get"): (
        "Get paginated transaction history for a wallet. "
        "Accepts optional query parameters: `limit`, `offset`, and `wallet_id`. "
        "Returns TransactionListResponse with an array of transactions "
        "including rjoules_delta, balance_after, and timestamp. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
    ("/api/wallet/withdraw", "post"): (
        "Initiate a withdrawal of rJoules as USDC to an external address. "
        "Requires a WithdrawRequest body with amount_rj, chain, to_address, "
        "private mode, and wallet_id. "
        "Returns WithdrawalResponse with amount_rj, chain, privacy mode, and tx_hash. "
        "Requires DelegationToken auth (P4 OCAP)."
    ),
}


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "docs/generated/openapi.json"
    with open(path, "r") as f:
        data = json.load(f)

    added = 0
    for url_path, methods in data.get("paths", {}).items():
        for method, spec in methods.items():
            if isinstance(spec, dict) and "description" not in spec:
                key = (url_path, method)
                if key in DESCRIPTIONS:
                    spec["description"] = DESCRIPTIONS[key]
                    added += 1
                    print(f"  + {method.upper()} {url_path}")
                else:
                    print(
                        f"  WARNING: No description defined for {method.upper()} {url_path}"
                    )

    with open(path, "w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")

    print(f"\nAdded {added} descriptions to {path}")


if __name__ == "__main__":
    main()
