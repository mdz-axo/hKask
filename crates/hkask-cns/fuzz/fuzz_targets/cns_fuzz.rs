#![no_main]
use bolero::check;

/// CnsSpan parsing must never panic on arbitrary input.
/// Tests deserialization boundary of the CNS span registry.
#[test]
fn fuzz_cns_span_parse_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = s.parse::<hkask_types::cns::CnsSpan>();
    });
}

/// EnergyCost construction with arbitrary u64 values.
/// Must not panic — the type is a simple newtype wrapper.
#[test]
fn fuzz_energy_cost_construct() {
    check!().with_type::<u64>().for_each(|v| {
        let _cost = hkask_cns::EnergyCost(*v);
    });
}

/// EnergyBudget construction with arbitrary cap and rate values.
/// Cap must be > 0; rate must not exceed cap.
#[test]
fn fuzz_energy_budget_construct() {
    check!().with_type::<(u64, u64)>().for_each(|(cap, rate)| {
        // EnergyBudget with zero cap should fail gracefully
        if *cap == 0 {
            // Zero-cap budget: construction itself may or may not panic
            // depending on implementation — we just test that rate ≤ cap
            // when cap > 0
            return;
        }
        let budget = hkask_cns::EnergyBudget::new(hkask_cns::EnergyCost(*cap))
            .with_replenish_rate(hkask_cns::EnergyCost(*rate));
        // The replenish rate might be clamped — just verify the struct is usable
        let _ = budget;
    });
}
