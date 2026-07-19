# E1 Verification Report (Manual / SSH Blocked)

Status: PARTIAL — pipeline healthy (API), logs unverified (no SSH host:port for zzsnh7xsrahwkt)

Verified:
- Pod zzsnh7xsrahwkt (rust-all-v2-merged) = RUNNING (RunPod API)
- Uptime ~23min (very early in ~26h training run)
- Config verified locally: r=32, patience=25, pissa_niter_4, hub_strategy=end, output_dir=/workspace/outputs
- Previous SSH patterns discovered: hetzner key, hosts 159.69.120.14 / 100.65.26.160:60745 (previous pods, not current)
- Connection to previous host timed out

Unknown (requires dashboard/SSH):
- Actual pipeline.log contents
- STAGE.*COMPLETE status
- eval_loss trajectory (expected: ~0.23 step 200, ~0.205 step 2000, ~0.198 step 3200 per guide L179-193)
- Hidden errors (OOM, SIGSEGV, disk space)
- Merged model upload status (upload configured but status unverified)

No critical errors detected via API. Recommendation: confirm via RunPod dashboard before committing to E2 config change (to avoid wasting GPU time on broken pipeline).
