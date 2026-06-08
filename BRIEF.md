# Brief — hKask Service Layer, Session 12

The 9-task service layer extraction and all post-extraction questions are **complete** (12 sessions, 46 key decisions, 51 tests). This session audited all remaining Tier 1 MEDIUM open questions — every one resolved as "by design" via depth test and constraint analysis. No MEDIUM-priority open questions remain.

## Session 12 Results

All 7 Tier 1 MEDIUM open questions audited and closed:

| ID | Question | Verdict | Key Reason |
|----|----------|---------|------------|
| F2 | Session lifecycle across surfaces | By design | CLI and API have fundamentally different session models; shared parts already extracted via EnsembleService |
| F3 | Unified authentication context | By design | Three surfaces have fundamentally different auth models; unified AuthContext would be shallow data-only container (fails depth test) |
| F6 | REPL vs API state boundary | Boundary documented | Shared fields already in ServiceContext; surface-specific fields correctly placed; boundary table written |
| F14 | Dual error mapping in API | Legitimately surface | All remaining direct ApiError constructions are legitimate HTTP-layer concerns (input validation, OCAP gates, auth, surface-only entities) |
| F17 | CuratorService standalone commands open DB each time | By design | P1 Prohibition protects standalone CLI pattern; single SQLite open per one-shot command is negligible |
| F18 | EnsembleService standing session extraction | By design | CLI/API standing session divergence wider than documented; 2-line common logic too shallow to extract |
| F19 | EnsembleService improv operation extraction | By design | Improv operations are CLI-only with no API counterpart; no duplication to extract |

## Current State

- **51 tests passing** (unchanged — design session, no code changes)
- **0 MEDIUM or HIGH open questions** remain
- **6 LOW/track-only questions** remain (F1, F8, F11, F12, F16, F22)
- **No further service layer extraction work is warranted** — every candidate has been audited against the depth test and constraint forces

## Key Files Updated

- `OPEN_QUESTIONS.md` — F2/F3/F6/F14/F17/F18/F19 all closed with audit findings
- `HANDOFF.md` — Session 12 added to history; Section 6 (what remains) updated; Section 7 (open questions) updated

## Mandatory Skills

`refactor-service-layer`, `coding-guidelines`, `tdd`, `constraint-forces`, `zoom-out`, `improve-codebase-architecture`, `diagnose`, `handoff`.