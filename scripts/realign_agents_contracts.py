#!/usr/bin/env python3
"""Realign hkask-agents contract IDs from legacy format to P{N}-agt-* format.

Maps each legacy AGT/BOT/ACP/consent/etc. ID to a domain-tagged new ID with the
appropriate motivating principle prefix. Production contracts get principle
annotations; test/integration comments keep only the new ID and description.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATE_DIR = ROOT / "crates" / "hkask-agents"

ID_MAP = {
    # ── ACP / capability protocol (P4) ──
    "AGT-073": "P4-agt-acp-audit-new",
    "AGT-074": "P4-agt-acp-audit-append",
    "AGT-075": "P4-agt-acp-message-visit",
    "AGT-076": "P4-agt-acp-message-sender",
    "AGT-077": "P4-agt-acp-message-id",
    "AGT-078": "P4-agt-acp-message-type",
    "AGT-079": "P4-agt-acp-runtime-new",
    "AGT-080": "P4-agt-acp-secret-derive",
    "AGT-081": "P4-agt-acp-token-issue",
    "AGT-082": "P4-agt-acp-agent-unregister",
    "AGT-083": "P4-agt-acp-agents-restore",
    "AGT-084": "P4-agt-acp-agents-list",
    "AGT-085": "P4-agt-acp-root-new",
    "AGT-086": "P4-agt-acp-root-token-issue",
    # ── Curator (P9) ──
    "AGT-049": "P9-agt-curator-persona-check",
    "AGT-050": "P9-agt-curator-persona-strip",
    "AGT-051": "P9-agt-curator-loop-new",
    "AGT-052": "P9-agt-curator-loop-new-with-consolidation",
    "AGT-053": "P9-agt-curator-loop-inbox",
    "AGT-054": "P9-agt-curator-loop-context",
    "AGT-055": "P9-agt-curator-loop-handle",
    "AGT-056": "P9-agt-curator-loop-restore-cursor",
    "AGT-057": "P9-agt-curator-context-new",
    "AGT-058": "P9-agt-curator-context-with-store",
    "AGT-059": "P9-agt-curator-context-with-acp",
    "AGT-060": "P9-agt-curator-context-handle",
    "AGT-061": "P9-agt-curator-context-directive",
    # ── Curator Agent / Spec Curator / Metacognition (P9) ──
    "AGT-088": "P9-agt-curator-agent-escalation-check",
    "AGT-089": "P9-agt-curator-agent-meta-new",
    "AGT-090": "P9-agt-curator-agent-tick",
    "AGT-091": "P9-agt-curator-agent-summary",
    "AGT-092": "P9-agt-curator-agent-direct",
    "AGT-093": "P9-agt-curator-agent-issue-directive",
    "AGT-094": "P9-agt-curator-agent-new",
    "AGT-095": "P9-agt-curator-agent-new-with-config",
    "AGT-096": "P9-agt-curator-agent-new-with-consolidation",
    "AGT-097": "P9-agt-curator-agent-curation-loop",
    "AGT-098": "P9-agt-curator-agent-metacognition-loop",
    "AGT-099": "P9-agt-curator-agent-context",
    "AGT-100": "P9-agt-curator-agent-spec-curator",
    "AGT-101": "P9-agt-curator-agent-spec-new",
    "AGT-102": "P9-agt-curator-agent-spec-calibrate",
    "AGT-103": "P9-agt-curator-agent-spec-with-config",
    "AGT-104": "P9-agt-curator-agent-spec-drift-threshold",
    "AGT-105": "P9-agt-curator-agent-spec-with-sink",
    "AGT-106": "P9-agt-curator-agent-spec-channel",
    "AGT-107": "P9-agt-curator-agent-spec-check",
    # ── Bot Health (P9) ──
    "BOT-HEALTH-001": "P9-agt-bot-health-classify",
    # ── Loop System (P9) ──
    "AGT-062": "P9-agt-loop-id",
    "AGT-063": "P9-agt-loop-system-new",
    "AGT-064": "P9-agt-loop-system-interval",
    "AGT-065": "P9-agt-loop-system-register",
    "AGT-066": "P9-agt-loop-system-cancel-token",
    "AGT-067": "P9-agt-loop-system-run",
    "AGT-068": "P9-agt-loop-system-tick",
    "AGT-069": "P9-agt-loop-system-run-ticks",
    "AGT-070": "P9-agt-loop-system-stop",
    "AGT-071": "P9-agt-loop-system-count",
    "AGT-072": "P9-agt-loop-system-ids",
    # ── Consent (P2) ──
    "AGT-038": "P2-agt-consent-record-new",
    "AGT-039": "P2-agt-consent-record-grant",
    "AGT-040": "P2-agt-consent-record-revoke",
    "AGT-041": "P2-agt-consent-record-is-active",
    "AGT-042": "P2-agt-consent-record-has-category",
    "AGT-043": "P2-agt-consent-manager-new",
    "AGT-044": "P2-agt-consent-manager-with-sink",
    "AGT-045": "P2-agt-consent-manager-grant",
    "AGT-046": "P2-agt-consent-manager-revoke",
    "AGT-047": "P2-agt-consent-manager-check",
    "AGT-048": "P2-agt-consent-manager-granted-categories",
    # ── Sovereignty (P1) ──
    "AGT-119": "P1-agt-sovereignty-checker-new",
    "AGT-120": "P1-agt-sovereignty-checker-can-access",
    "AGT-121": "P1-agt-sovereignty-checker-can-perform",
    # ── Memory ports & adapter (P3) ──
    "AGT-032": "P3-agt-memory-request-new",
    "AGT-033": "P3-agt-memory-request-episodic",
    "AGT-034": "P3-agt-memory-request-semantic",
    "AGT-035": "P3-agt-memory-confidence-map",
    "AGT-036": "P3-agt-memory-recall-episodic",
    "AGT-037": "P3-agt-memory-recall-semantic",
    "AGT-109": "P3-agt-memory-adapter-new",
    "AGT-110": "P3-agt-memory-adapter-in-memory",
    "AGT-111": "P3-agt-memory-adapter-in-memory-unwrap",
    "AGT-112": "P3-agt-memory-adapter-encrypted",
    # ── Registry & MCP adapters (P3 / P4) ──
    "AGT-108": "P3-agt-registry-source-new",
    "AGT-113": "P4-agt-mcp-capability-adapter-new",
    "AGT-114": "P4-agt-mcp-full-adapter-new",
    "AGT-115": "P3-agt-registry-loader-new",
    "AGT-116": "P3-agt-registry-loader-restore",
    "AGT-117": "P3-agt-registry-loader-load",
    "AGT-118": "P3-agt-registry-loader-store",
    # ── Prompt analysis (P9) ──
    "AGT-087": "P9-agt-prompt-classify",
    # ── Pod lifecycle (P1) ──
    "AGT-122": "P1-agt-pod-new",
    "AGT-123": "P1-agt-pod-register",
    "AGT-124": "P1-agt-pod-activate",
    "AGT-125": "P1-agt-pod-deactivate",
    "AGT-126": "P1-agt-pod-delegate",
    "AGT-127": "P1-agt-pod-is-active",
    "AGT-128": "P1-agt-pod-state",
    "AGT-129": "P1-agt-pod-enter-server-mode",
    "AGT-130": "P1-agt-pod-enter-chat-mode",
    "AGT-131": "P1-agt-pod-exit-mode",
    "AGT-132": "P1-agt-pod-is-server-mode",
    "AGT-133": "P1-agt-pod-set-voice",
    "AGT-134": "P1-agt-pod-get-voice",
    "AGT-135": "P1-agt-pod-voice-description",
    "AGT-136": "P1-agt-pod-is-chat-mode",
    "AGT-137": "P1-agt-pod-check-sovereignty",
    # ── PodManager (P1) ──
    "AGT-138": "P1-agt-pod-manager-new",
    "AGT-139": "P1-agt-pod-manager-with-consent",
    "AGT-140": "P1-agt-pod-manager-activation-hook",
    "AGT-141": "P1-agt-pod-manager-with-checker",
    "AGT-142": "P1-agt-pod-manager-with-sink",
    "AGT-143": "P1-agt-pod-manager-with-governed-tool",
    "AGT-144": "P1-agt-pod-manager-with-ports",
    "AGT-145": "P1-agt-pod-manager-inference-port",
    "AGT-146": "P1-agt-pod-manager-sovereignty-checker",
    "AGT-147": "P1-agt-pod-manager-default",
    "AGT-148": "P1-agt-pod-manager-create-pod",
    "AGT-149": "P1-agt-pod-manager-activate-pod",
    "AGT-150": "P1-agt-pod-manager-deactivate-pod",
    "AGT-151": "P1-agt-pod-manager-recall-lifecycle",
    "AGT-152": "P1-agt-pod-manager-status",
    "AGT-153": "P1-agt-pod-manager-list-status",
    "AGT-154": "P1-agt-pod-manager-acp-port",
    "AGT-155": "P1-agt-pod-manager-find-by-name",
    "AGT-156": "P1-agt-pod-manager-webid",
    "AGT-157": "P1-agt-pod-manager-has-role",
    "AGT-158": "P1-agt-pod-manager-has-capability",
    "AGT-159": "P1-agt-pod-manager-assign-role",
    "AGT-160": "P1-agt-pod-manager-set-mode",
    # ── Pod types (P4 + P7) ──
    "AGT-161": "P4-agt-pod-lifecycle-can-transition",
    # ── ACP tests (P4) ──
    "acp-wildcard-001": "P4-agt-acp-wildcard-reject-test",
    "acp-wildcard-002": "P4-agt-acp-wildcard-mixed-reject-test",
    "acp-register-001": "P4-agt-acp-register-test",
    "acp-register-002": "P4-agt-acp-register-dup-test",
    "acp-register-003": "P4-agt-acp-register-capabilities-test",
    "acp-unregister-001": "P4-agt-acp-unregister-test",
    "acp-unregister-002": "P4-agt-acp-unregister-unknown-test",
    "acp-revoke-001": "P4-agt-acp-revoke-test",
    "acp-restore-001": "P4-agt-acp-restore-test",
    "acp-list-001": "P4-agt-acp-list-test",
    "acp-list-002": "P4-agt-acp-list-empty-test",
    # ── Consent tests (P2) ──
    "P2-consent-record-001": "P2-agt-consent-record-new-test",
    "P2-consent-record-002": "P2-agt-consent-record-grant-test",
    "P2-consent-record-003": "P2-agt-consent-record-revoke-test",
    "P2-consent-record-004": "P2-agt-consent-record-has-category-test",
    # ── Persona filter tests (P4) ──
    "persona-filter-001": "P4-agt-persona-filter-non-ascii-check-test",
    "persona-filter-002": "P4-agt-persona-filter-non-ascii-strip-test",
    "persona-filter-003": "P4-agt-persona-filter-ascii-detect-test",
    "persona-filter-004": "P4-agt-persona-filter-clean-test",
    # ── Pod dual-gate tests (P4) ──
    "P4-dual-gate": "P4-agt-pod-dual-gate-test",
    # ── Pod types tests (P1) ──
    "types-pod-001": "P1-agt-pod-lifecycle-transition-test",
    "types-pod-002": "P1-agt-pod-lifecycle-invalid-test",
    "types-pod-003": "P1-agt-pod-new-defaults-test",
    "types-pod-004": "P1-agt-pod-is-active-test",
    "types-pod-005": "P1-agt-pod-voice-roundtrip-test",
    "types-pod-006": "P1-agt-pod-error-display-test",
    # ── Sovereignty tests (P1) ──
    "P1-sovereignty-001": "P1-agt-sovereignty-deny-all-test",
    "P1-sovereignty-002": "P1-agt-sovereignty-boundary-test",
    # ── Integration tests (P1) ──
    "INT-005.1": "P1-agt-pod-integration-create-test",
    "INT-005.2": "P1-agt-pod-integration-list-test",
    "INT-005.3": "P1-agt-pod-integration-mode-test",
    "INT-005.4": "P1-agt-pod-integration-deactivate-test",
    "INT-005.5": "P1-agt-pod-integration-not-found-test",
    "INT-005.6": "P1-agt-pod-integration-inference-test",
}

# Principle annotations for production contract IDs (no `-test` suffix).
PRINCIPLE_ANNOTATIONS = {
    "P4-agt-acp-audit-new": [
        "[P4] Motivating: Clear Boundaries — audit log attests capability actions",
        "[P1] Constraining: User Sovereignty — every action is attributable to an agent",
    ],
    "P4-agt-acp-audit-append": [
        "[P4] Motivating: Clear Boundaries — append-only audit preserves OCAP evidence",
        "[P8] Constraining: Semantic Grounding — entries are structured and traceable",
    ],
    "P4-agt-acp-message-visit": [
        "[P4] Motivating: Clear Boundaries — single dispatch site for A2A message variants",
    ],
    "P4-agt-acp-message-sender": [
        "[P4] Motivating: Clear Boundaries — sender identity is explicit per variant",
        "[P1] Constraining: User Sovereignty — identity belongs to the agent/user",
    ],
    "P4-agt-acp-message-id": [
        "[P4] Motivating: Clear Boundaries — correlation/artifact IDs enable traceability",
    ],
    "P4-agt-acp-message-type": [
        "[P8] Motivating: Semantic Grounding — stable message type labels",
    ],
    "P4-agt-acp-runtime-new": [
        "[P4] Motivating: Clear Boundaries — ACP runtime derives root authority from master secret",
        "[P1] Constraining: User Sovereignty — root WebID is user-derived",
    ],
    "P4-agt-acp-secret-derive": [
        "[P4] Motivating: Clear Boundaries — HKDF isolates per-agent secrets",
        "[P1] Constraining: User Sovereignty — secrets are bound to agent identity",
    ],
    "P4-agt-acp-token-issue": [
        "[P4] Motivating: Clear Boundaries — DelegationToken attenuates capabilities",
        "[P1] Constraining: User Sovereignty — tokens are issued to named agents",
    ],
    "P4-agt-acp-agent-unregister": [
        "[P4] Motivating: Clear Boundaries — unregister revokes all agent capabilities",
    ],
    "P4-agt-acp-agents-restore": [
        "[P4] Motivating: Clear Boundaries — restore preserves capability graph",
    ],
    "P4-agt-acp-agents-list": [
        "[P4] Motivating: Clear Boundaries — enumerate registered agents",
    ],
    "P4-agt-acp-root-new": [
        "[P4] Motivating: Clear Boundaries — root authority is the capability issuer",
    ],
    "P4-agt-acp-root-token-issue": [
        "[P4] Motivating: Clear Boundaries — root tokens start the delegation chain",
        "[P7] Constraining: Evolutionary Architecture — attenuation limits emerged from usage",
    ],
    "P9-agt-curator-persona-check": [
        "[P9] Motivating: Homeostatic Self-Regulation — persona filter prevents harmful output",
        "[P4] Constraining: Clear Boundaries — forbidden patterns are explicit",
    ],
    "P9-agt-curator-persona-strip": [
        "[P9] Motivating: Homeostatic Self-Regulation — stripping reduces harm while preserving utility",
    ],
    "P9-agt-curator-loop-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — Curation Loop is the regulatory sense-act loop",
        "[P4] Constraining: Clear Boundaries — single CuratorHandle capability",
    ],
    "P9-agt-curator-loop-new-with-consolidation": [
        "[P9] Motivating: Homeostatic Self-Regulation — consolidation tunes the loop",
        "[P7] Constraining: Evolutionary Architecture — consolidation config emerged from usage",
    ],
    "P9-agt-curator-loop-inbox": [
        "[P9] Motivating: Homeostatic Self-Regulation — unified inbox receives CurationInput",
    ],
    "P9-agt-curator-loop-context": [
        "[P9] Motivating: Homeostatic Self-Regulation — context exposes CNS and escalation",
    ],
    "P9-agt-curator-loop-handle": [
        "[P9] Motivating: Homeostatic Self-Regulation — handle is the capability to curate",
    ],
    "P9-agt-curator-loop-restore-cursor": [
        "[P9] Motivating: Homeostatic Self-Regulation — cursor restore avoids re-processing history",
    ],
    "P9-agt-curator-context-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — CuratorContext bundles regulatory dependencies",
    ],
    "P9-agt-curator-context-with-store": [
        "[P9] Motivating: Homeostatic Self-Regulation — NuEvent store enables algedonic review",
    ],
    "P9-agt-curator-context-with-acp": [
        "[P4] Motivating: Clear Boundaries — ACP port lets Curator direct bots",
    ],
    "P9-agt-curator-context-handle": [
        "[P9] Motivating: Homeostatic Self-Regulation — accessor for the Curator capability handle",
    ],
    "P9-agt-curator-context-directive": [
        "[P9] Motivating: Homeostatic Self-Regulation — issue directives to the Curation Loop",
    ],
    "P9-agt-curator-agent-escalation-check": [
        "[P9] Motivating: Homeostatic Self-Regulation — escalation policy classifies variety deficit",
        "[P4] Constraining: Clear Boundaries — thresholds define explicit boundaries",
    ],
    "P9-agt-curator-agent-meta-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — MetacognitionLoop monitors agent health",
    ],
    "P9-agt-curator-agent-tick": [
        "[P9] Motivating: Homeostatic Self-Regulation — tick produces latest HealthSnapshot",
    ],
    "P9-agt-curator-agent-summary": [
        "[P9] Motivating: Homeostatic Self-Regulation — summary posts system state to standing session",
    ],
    "P9-agt-curator-agent-direct": [
        "[P9] Motivating: Homeostatic Self-Regulation — direct a bot to take corrective action",
    ],
    "P9-agt-curator-agent-issue-directive": [
        "[P9] Motivating: Homeostatic Self-Regulation — delegate directive to CuratorContext",
    ],
    "P9-agt-curator-agent-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — CuratorAgent composes Curation + Metacognition",
    ],
    "P9-agt-curator-agent-new-with-config": [
        "[P9] Motivating: Homeostatic Self-Regulation — custom metacognition configuration",
        "[P7] Constraining: Evolutionary Architecture — thresholds emerge from real usage",
    ],
    "P9-agt-curator-agent-new-with-consolidation": [
        "[P9] Motivating: Homeostatic Self-Regulation — consolidation wired into CuratorAgent",
    ],
    "P9-agt-curator-agent-curation-loop": [
        "[P9] Motivating: Homeostatic Self-Regulation — accessor for the pure regulatory loop",
    ],
    "P9-agt-curator-agent-metacognition-loop": [
        "[P9] Motivating: Homeostatic Self-Regulation — accessor for the persona/agent loop",
    ],
    "P9-agt-curator-agent-context": [
        "[P9] Motivating: Homeostatic Self-Regulation — accessor for capability-disciplined context",
    ],
    "P9-agt-curator-agent-spec-curator": [
        "[P9] Motivating: Homeostatic Self-Regulation — DefaultSpecCurator detects specification drift",
    ],
    "P9-agt-curator-agent-spec-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — initialize spec curator with coherence threshold",
        "[P7] Constraining: Evolutionary Architecture — thresholds calibrated from observations",
    ],
    "P9-agt-curator-agent-spec-calibrate": [
        "[P9] Motivating: Homeostatic Self-Regulation — calibrate threshold from historical coherence",
        "[P7] Constraining: Evolutionary Architecture — 25th-percentile heuristic emerged from usage",
    ],
    "P9-agt-curator-agent-spec-with-config": [
        "[P9] Motivating: Homeostatic Self-Regulation — apply explicit curation threshold config",
    ],
    "P9-agt-curator-agent-spec-drift-threshold": [
        "[P9] Motivating: Homeostatic Self-Regulation — drift threshold triggers escalation",
    ],
    "P9-agt-curator-agent-spec-with-sink": [
        "[P9] Motivating: Homeostatic Self-Regulation — emit algedonic events on drift escalation",
    ],
    "P9-agt-curator-agent-spec-channel": [
        "[P9] Motivating: Homeostatic Self-Regulation — wire spec events into CurationLoop",
    ],
    "P9-agt-curator-agent-spec-check": [
        "[P9] Motivating: Homeostatic Self-Regulation — check spec coherence and emit drift alerts",
    ],
    "P9-agt-bot-health-classify": [
        "[P9] Motivating: Homeostatic Self-Regulation — classify bot energy health for Curator",
        "[P4] Constraining: Clear Boundaries — thresholds map consumption ratio to status",
    ],
    "P9-agt-loop-id": [
        "[P8] Motivating: Semantic Grounding — LoopId names the regulatory loops",
    ],
    "P9-agt-loop-system-new": [
        "[P9] Motivating: Homeostatic Self-Regulation — LoopSystem orchestrates sense-act cycles",
        "[P5] Constraining: Essentialism — minimal registry + cancellation token",
    ],
    "P9-agt-loop-system-interval": [
        "[P9] Motivating: Homeostatic Self-Regulation — configurable tick interval per loop",
        "[P7] Constraining: Evolutionary Architecture — intervals emerge from operational need",
    ],
    "P9-agt-loop-system-register": [
        "[P9] Motivating: Homeostatic Self-Regulation — register loop instances under LoopId",
    ],
    "P9-agt-loop-system-cancel-token": [
        "[P9] Motivating: Homeostatic Self-Regulation — cancellation token stops all loops",
    ],
    "P9-agt-loop-system-run": [
        "[P9] Motivating: Homeostatic Self-Regulation — spawn tokio tasks for each loop",
    ],
    "P9-agt-loop-system-tick": [
        "[P9] Motivating: Homeostatic Self-Regulation — single sense-compare-compute-act tick",
    ],
    "P9-agt-loop-system-run-ticks": [
        "[P9] Motivating: Homeostatic Self-Regulation — run multiple ticks sequentially",
    ],
    "P9-agt-loop-system-stop": [
        "[P9] Motivating: Homeostatic Self-Regulation — idempotent stop signal",
    ],
    "P9-agt-loop-system-count": [
        "[P8] Motivating: Semantic Grounding — count of registered loop instances",
    ],
    "P9-agt-loop-system-ids": [
        "[P8] Motivating: Semantic Grounding — list registered loop IDs",
    ],
    "P2-agt-consent-record-new": [
        "[P2] Motivating: Affirmative Consent — consent record starts empty and active",
        "[P1] Constraining: User Sovereignty — record is bound to user WebID",
    ],
    "P2-agt-consent-record-grant": [
        "[P2] Motivating: Affirmative Consent — explicit grant adds a data category",
    ],
    "P2-agt-consent-record-revoke": [
        "[P2] Motivating: Affirmative Consent — revocation terminates consent",
    ],
    "P2-agt-consent-record-is-active": [
        "[P2] Motivating: Affirmative Consent — active iff not revoked",
    ],
    "P2-agt-consent-record-has-category": [
        "[P2] Motivating: Affirmative Consent — category check enforces scoped grant",
    ],
    "P2-agt-consent-manager-new": [
        "[P2] Motivating: Affirmative Consent — manager caches active consent records",
    ],
    "P2-agt-consent-manager-with-sink": [
        "[P9] Motivating: Homeostatic Self-Regulation — CNS instrumentation for denials (observability only, no feedback)",
    ],
    "P2-agt-consent-manager-grant": [
        "[P2] Motivating: Affirmative Consent — persist a scoped grant",
    ],
    "P2-agt-consent-manager-revoke": [
        "[P2] Motivating: Affirmative Consent — revoke all consent for a WebID",
    ],
    "P2-agt-consent-manager-check": [
        "[P2] Motivating: Affirmative Consent — terminal deny unless active grant exists",
        "[P1] Constraining: User Sovereignty — check is per-user/data-category",
    ],
    "P2-agt-consent-manager-granted-categories": [
        "[P2] Motivating: Affirmative Consent — list granted categories for disclosure",
    ],
    "P1-agt-sovereignty-checker-new": [
        "[P1] Motivating: User Sovereignty — checker enforces the user-data boundary",
        "[P2] Constraining: Affirmative Consent — delegates to consent port",
    ],
    "P1-agt-sovereignty-checker-can-access": [
        "[P1] Motivating: User Sovereignty — access decision combines consent + ownership",
    ],
    "P1-agt-sovereignty-checker-can-perform": [
        "[P1] Motivating: User Sovereignty — action decision combines consent + operation",
    ],
    "P3-agt-memory-request-new": [
        "[P3] Motivating: Generative Space — StorageRequest creates a memory triple",
        "[P1] Constraining: User Sovereignty — access.owner_webid carries ownership",
    ],
    "P3-agt-memory-request-episodic": [
        "[P3] Motivating: Generative Space — episodic request binds perspective to owner",
    ],
    "P3-agt-memory-request-semantic": [
        "[P3] Motivating: Generative Space — semantic request is perspective-free",
    ],
    "P3-agt-memory-confidence-map": [
        "[P8] Motivating: Semantic Grounding — classification maps to confidence scalar",
    ],
    "P3-agt-memory-recall-episodic": [
        "[P3] Motivating: Generative Space — episodic recall requires delegation token",
        "[P4] Constraining: Clear Boundaries — token proves capability",
    ],
    "P3-agt-memory-recall-semantic": [
        "[P3] Motivating: Generative Space — semantic recall requires delegation token",
        "[P4] Constraining: Clear Boundaries — token proves capability",
    ],
    "P3-agt-memory-adapter-new": [
        "[P3] Motivating: Generative Space — MemoryLoopForwarder wires episodic + semantic",
    ],
    "P3-agt-memory-adapter-in-memory": [
        "[P3] Motivating: Generative Space — in-memory SQLite adapter for tests",
    ],
    "P3-agt-memory-adapter-in-memory-unwrap": [
        "[P3] Motivating: Generative Space — infallible in-memory constructor for tests",
    ],
    "P3-agt-memory-adapter-encrypted": [
        "[P1] Motivating: User Sovereignty — encrypted on-disk memory adapter",
        "[P4] Constraining: Clear Boundaries — passphrase protects the store",
    ],
    "P3-agt-registry-source-new": [
        "[P5] Motivating: Essentialism — filesystem registry source is a unit struct",
    ],
    "P4-agt-mcp-capability-adapter-new": [
        "[P4] Motivating: Clear Boundaries — capability-only adapter gates tools without runtime",
    ],
    "P4-agt-mcp-full-adapter-new": [
        "[P4] Motivating: Clear Boundaries — full adapter combines capability checker + MCP runtime",
    ],
    "P3-agt-registry-loader-new": [
        "[P3] Motivating: Generative Space — loader reads YAML agent definitions into registry",
    ],
    "P3-agt-registry-loader-restore": [
        "[P3] Motivating: Generative Space — restore previously registered agents",
    ],
    "P3-agt-registry-loader-load": [
        "[P3] Motivating: Generative Space — load agent definitions from filesystem",
    ],
    "P3-agt-registry-loader-store": [
        "[P8] Motivating: Semantic Grounding — accessor for the registry store",
    ],
    "P9-agt-prompt-classify": [
        "[P9] Motivating: Homeostatic Self-Regulation — classify prompt to guide loop action",
    ],
    "P1-agt-pod-new": [
        "[P1] Motivating: User Sovereignty — AgentPod is the user's agent container",
        "[P4] Constraining: Clear Boundaries — OCAP secret + capability token on creation",
    ],
    "P1-agt-pod-register": [
        "[P1] Motivating: User Sovereignty — register pod with ACP under its WebID",
    ],
    "P1-agt-pod-activate": [
        "[P1] Motivating: User Sovereignty — activate grants MCP access",
        "[P4] Constraining: Clear Boundaries — requires Registered state",
    ],
    "P1-agt-pod-deactivate": [
        "[P1] Motivating: User Sovereignty — deactivate terminates MCP access",
    ],
    "P1-agt-pod-delegate": [
        "[P4] Motivating: Clear Boundaries — delegate capability to another holder with attenuation",
        "[P7] Constraining: Evolutionary Architecture — attenuation limit emerged from usage",
    ],
    "P1-agt-pod-is-active": [
        "[P8] Motivating: Semantic Grounding — state accessor for Activated",
    ],
    "P1-agt-pod-state": [
        "[P8] Motivating: Semantic Grounding — lifecycle state accessor",
    ],
    "P1-agt-pod-enter-server-mode": [
        "[P1] Motivating: User Sovereignty — enter server mode to serve MCP role",
        "[P4] Constraining: Clear Boundaries — requires Activated + assigned role",
    ],
    "P1-agt-pod-enter-chat-mode": [
        "[P1] Motivating: User Sovereignty — enter chat mode for interactive use",
        "[P4] Constraining: Clear Boundaries — requires Activated + no other mode",
    ],
    "P1-agt-pod-exit-mode": [
        "[P1] Motivating: User Sovereignty — exit current mode",
    ],
    "P1-agt-pod-is-server-mode": [
        "[P8] Motivating: Semantic Grounding — mode accessor",
    ],
    "P1-agt-pod-set-voice": [
        "[P3] Motivating: Generative Space — configure voice design",
    ],
    "P1-agt-pod-get-voice": [
        "[P8] Motivating: Semantic Grounding — voice design accessor",
    ],
    "P1-agt-pod-voice-description": [
        "[P8] Motivating: Semantic Grounding — return TTS description",
    ],
    "P1-agt-pod-is-chat-mode": [
        "[P8] Motivating: Semantic Grounding — mode accessor",
    ],
    "P1-agt-pod-check-sovereignty": [
        "[P1] Motivating: User Sovereignty — verify action against sovereignty/consent",
        "[P2] Constraining: Affirmative Consent — delegates to consent boundary",
    ],
    "P1-agt-pod-manager-new": [
        "[P1] Motivating: User Sovereignty — PodManager orchestrates user agent pods",
        "[P4] Constraining: Clear Boundaries — default DenyAllConsent",
    ],
    "P1-agt-pod-manager-with-consent": [
        "[P2] Constraining: Affirmative Consent — replace default consent policy",
    ],
    "P1-agt-pod-manager-activation-hook": [
        "[P3] Motivating: Generative Space — hook runs when pod becomes active",
    ],
    "P1-agt-pod-manager-with-checker": [
        "[P4] Constraining: Clear Boundaries — set capability checker",
    ],
    "P1-agt-pod-manager-with-sink": [
        "[P9] Motivating: Homeostatic Self-Regulation — attach CNS event sink",
    ],
    "P1-agt-pod-manager-with-governed-tool": [
        "[P4] Constraining: Clear Boundaries — wire governed tool for capability gating",
    ],
    "P1-agt-pod-manager-with-ports": [
        "[P1] Motivating: User Sovereignty — configure runtime ports for pods",
    ],
    "P1-agt-pod-manager-inference-port": [
        "[P8] Motivating: Semantic Grounding — accessor for inference port",
    ],
    "P1-agt-pod-manager-sovereignty-checker": [
        "[P1] Motivating: User Sovereignty — get per-pod sovereignty checker",
    ],
    "P1-agt-pod-manager-default": [
        "[P5] Motivating: Essentialism — default manager with in-memory mocks",
    ],
    "P1-agt-pod-manager-create-pod": [
        "[P1] Motivating: User Sovereignty — create a new agent pod from template + persona",
    ],
    "P1-agt-pod-manager-activate-pod": [
        "[P1] Motivating: User Sovereignty — activate pod (register + grant MCP)",
    ],
    "P1-agt-pod-manager-deactivate-pod": [
        "[P1] Motivating: User Sovereignty — deactivate pod and revoke capabilities",
    ],
    "P1-agt-pod-manager-recall-lifecycle": [
        "[P3] Motivating: Generative Space — recall pod lifecycle episodes",
    ],
    "P1-agt-pod-manager-status": [
        "[P8] Motivating: Semantic Grounding — get pod status",
    ],
    "P1-agt-pod-manager-list-status": [
        "[P8] Motivating: Semantic Grounding — list all pod statuses",
    ],
    "P1-agt-pod-manager-acp-port": [
        "[P4] Constraining: Clear Boundaries — accessor for ACP port",
    ],
    "P1-agt-pod-manager-find-by-name": [
        "[P8] Motivating: Semantic Grounding — lookup pod by replicant name",
    ],
    "P1-agt-pod-manager-webid": [
        "[P1] Motivating: User Sovereignty — get pod's WebID",
    ],
    "P1-agt-pod-manager-has-role": [
        "[P4] Constraining: Clear Boundaries — check MCP role assignment",
    ],
    "P1-agt-pod-manager-has-capability": [
        "[P4] Constraining: Clear Boundaries — check tool capability",
    ],
    "P1-agt-pod-manager-assign-role": [
        "[P1] Motivating: User Sovereignty — assign MCP role to named pod",
    ],
    "P1-agt-pod-manager-set-mode": [
        "[P1] Motivating: User Sovereignty — set pod mode (server/chat/exit)",
    ],
    "P4-agt-pod-lifecycle-can-transition": [
        "[P4] Motivating: Clear Boundaries — lifecycle state machine enforces transitions",
        "[P7] Constraining: Evolutionary Architecture — linear model + idempotent restate",
    ],
}


def is_test_id(new_id: str) -> bool:
    return new_id.endswith("-test")


def process_file(path: Path) -> tuple[str, int]:
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    changed = 0
    out = []

    for line in lines:
        new_line = line
        if re.search(r"^\s*(///|//|//!)\s*REQ:\s*", line):
            for old_id, new_id in ID_MAP.items():
                if old_id in new_line:
                    new_line = re.sub(
                        rf"(REQ:\s*){re.escape(old_id)}",
                        rf"\g<1>{new_id}",
                        new_line,
                        count=1,
                    )
                    changed += 1
                    if not is_test_id(new_id):
                        annotations = PRINCIPLE_ANNOTATIONS.get(new_id)
                        if annotations:
                            m = re.match(r"^(\s*///\s*)", line)
                            prefix = m.group(1) if m else "/// "
                            extra = "".join([f"{prefix}{a}\n" for a in annotations])
                            new_line = new_line + extra
                    break
        out.append(new_line)

    return "".join(out), changed


def main() -> None:
    src_dir = CRATE_DIR / "src"
    tests_dir = CRATE_DIR / "tests"
    files = list(src_dir.rglob("*.rs")) + list(tests_dir.rglob("*.rs"))

    total = 0
    for path in sorted(files):
        new_text, changed = process_file(path)
        if changed:
            path.write_text(new_text)
            print(f"{path.relative_to(ROOT)}: {changed} replacements")
            total += changed

    print(f"\nTotal replacements: {total}")


if __name__ == "__main__":
    main()
