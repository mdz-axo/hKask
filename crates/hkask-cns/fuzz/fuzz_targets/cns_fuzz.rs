use bolero::check;

/// CnsSpan parsing must never panic on arbitrary input.
#[test]
fn fuzz_cns_span_parse_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = s.parse::<hkask_types::cns::CnsSpan>();
    });
}

/// EnergyCost construction with arbitrary u64 values.
#[test]
fn fuzz_energy_cost_construct() {
    check!().with_type::<u64>().for_each(|v| {
        let _cost = hkask_cns::EnergyCost(*v);
    });
}

/// EnergyBudget construction with arbitrary cap and rate.
#[test]
fn fuzz_energy_budget_construct() {
    check!().with_type::<(u64, u64)>().for_each(|(cap, rate)| {
        if *cap == 0 {
            return;
        }
        let budget = hkask_cns::EnergyBudget::new(hkask_cns::EnergyCost(*cap))
            .with_replenish_rate(hkask_cns::EnergyCost(*rate));
        let _ = budget;
    });
}
