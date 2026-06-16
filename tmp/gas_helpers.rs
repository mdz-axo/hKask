fn classify_event_kind(event: &NuEvent) -> GasEventKind {
    match event.span.kind() {
        SpanKind::GasReserved => GasEventKind::Reserved,
        SpanKind::GasSettled => GasEventKind::Settled,
        SpanKind::GasDepleted => GasEventKind::Depleted,
    }
}

fn is_gas_event(event: &NuEvent) -> bool {
    matches!(
        event.span.kind(),
        SpanKind::GasReserved | SpanKind::GasSettled | SpanKind::GasDepleted
    )
}

fn extract_tool_name(event: &NuEvent) -> String {
    event
        .observation
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_cost(event: &NuEvent) -> u64 {
    event
        .observation
        .get("estimated_cost")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn extract_actual(event: &NuEvent) -> u64 {
    event
        .observation
        .get("actual")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}
