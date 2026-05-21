// Auto-extracted inline tests for hkask-cns
// Extracted: Thu May 21 00:22:24 PDT 2026

// === From energy.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{CapabilityAction, CapabilityResource};

    #[test]
    fn test_energy_span_type_as_str() {
        assert_eq!(EnergySpanType::Allocate.as_str(), "cns.energy.allocate");
        assert_eq!(EnergySpanType::Consume.as_str(), "cns.energy.consume");
        assert_eq!(
            EnergySpanType::Opportunity.as_str(),
            "cns.energy.opportunity"
        );
        assert_eq!(EnergySpanType::Deficit.as_str(), "cns.energy.deficit");
    }

    #[test]
    fn test_energy_span_type_parse() {
        assert_eq!(
            EnergySpanType::parse_str("allocate"),
            Some(EnergySpanType::Allocate)
        );
        assert_eq!(
            EnergySpanType::parse_str("cns.energy.consume"),
            Some(EnergySpanType::Consume)
        );
        assert_eq!(EnergySpanType::parse_str("invalid"), None);
    }

    #[test]
    fn test_energy_budget_new() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.cap, 10000);
        assert_eq!(budget.remaining, 10000);
        assert_eq!(budget.cost_per_token, 0.25);
        assert_eq!(budget.alert_threshold, 0.8);
        assert!(budget.hard_limit);
    }

    #[test]
    fn test_energy_budget_calculate_cost() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.calculate_cost(100), 25);
        assert_eq!(budget.calculate_cost(1000), 250);
    }

    #[test]
    fn test_energy_budget_calculate_tokens() {
        let budget = EnergyBudget::new(10000);
        assert_eq!(budget.calculate_tokens(25), 100);
        assert_eq!(budget.calculate_tokens(250), 1000);
    }

    #[test]
    fn test_energy_budget_allocate() {
        let mut budget = EnergyBudget::new(10000);
        let cost = budget.allocate(1000).unwrap();
        assert_eq!(cost, 250);
        assert_eq!(budget.remaining, 9750);
    }

    #[test]
    fn test_energy_budget_allocate_exceeded() {
        let mut budget = EnergyBudget::new(100);
        let result = budget.allocate(1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_energy_budget_should_alert() {
        let mut budget = EnergyBudget::new(10000);
        assert!(!budget.should_alert());

        // Allocate enough to reach 80% usage (8000 energy units = 32000 tokens)
        budget.allocate(32000).unwrap();
        assert!(budget.should_alert());
    }

    #[test]
    fn test_energy_budget_usage_ratio() {
        let mut budget = EnergyBudget::new(10000);
        assert_eq!(budget.usage_ratio(), 0.0);

        // Allocate 5000 tokens = 1250 energy units
        budget.allocate(5000).unwrap();
        assert!((budget.usage_ratio() - 0.125).abs() < 0.01);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2);
        assert_eq!(estimate_tokens("hello world"), 3);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_calculate_energy_cost() {
        // "hello" = 5 chars / 4 = 1.25 tokens, ceil = 2, * 0.25 = 0.5, rounded = 0
        assert_eq!(calculate_energy_cost("hello", 0.25), 0);
        // "hello world" = 11 chars / 4 = 2.75 tokens, ceil = 3, * 0.25 = 0.75, rounded = 0
        assert_eq!(calculate_energy_cost("hello world", 0.25), 0);
    }

    #[test]
    fn test_energy_account_new() {
        let account = EnergyAccount::new("test", 10000);
        assert_eq!(account.id, "test");
        assert_eq!(account.budget.cap, 10000);
        assert_eq!(account.total_allocated, 0);
        assert_eq!(account.total_consumed, 0);
    }

    #[test]
    fn test_energy_account_allocate() {
        let mut account = EnergyAccount::new("test", 10000);
        let cost = account.allocate(1000).unwrap();
        assert_eq!(cost, 250);
        assert_eq!(account.total_allocated, 250);
    }

    #[test]
    fn test_energy_account_consume() {
        let mut account = EnergyAccount::new("test", 10000);
        account.consume(100);
        assert_eq!(account.total_consumed, 100);
    }

    #[test]
    fn test_opportunity_cost() {
        let cost = OpportunityCost::new("test", 100, 150);
        assert_eq!(cost.operation, "test");
        assert_eq!(cost.actual_cost, 100);
        assert_eq!(cost.alternative_cost, 150);
        assert_eq!(cost.cost, 50);
    }

    #[test]
    fn test_energy_account_opportunity() {
        let mut account = EnergyAccount::new("test", 10000);
        account.record_opportunity(OpportunityCost::new("test", 100, 150));
        assert_eq!(account.total_opportunity_cost(), 50);
    }

    #[test]
    fn test_cleanup_expired_capabilities() {
        let secret = b"test-secret";
        let from = WebID::new();
        let to = WebID::new();
        let mut valid_cap = hkask_types::CapabilityToken::new(
            CapabilityResource::Tool,
            "test".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        valid_cap.expires_at = Some(2000);
        let mut expired_cap = hkask_types::CapabilityToken::new(
            CapabilityResource::Tool,
            "test2".to_string(),
            CapabilityAction::Execute,
            from.clone(),
            to.clone(),
            secret,
        );
        expired_cap.expires_at = Some(500);
        let capabilities = vec![valid_cap, expired_cap];
        let (kept, removed) = cleanup_expired_capabilities(&capabilities, 1000);
        assert_eq!(kept, 1);
        assert_eq!(removed, 1);
        let (kept, removed) = cleanup_expired_capabilities(&capabilities, 100);
        assert_eq!(kept, 2);
        assert_eq!(removed, 0);
        let (kept, removed) = cleanup_expired_capabilities(&capabilities, 3000);
        assert_eq!(kept, 0);
        assert_eq!(removed, 2);
    }

    #[test]
    fn test_recommended_cleanup_interval() {
        assert_eq!(recommended_cleanup_interval(), 300);
    }
}

// === From review_queue.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_queue_new() {
        let queue = ReviewQueue::new();
        assert_eq!(queue.pending_violations().len(), 0);
    }

    #[test]
    fn test_review_queue_add_violation() {
        let mut queue = ReviewQueue::new();
        let agent_id = WebID::new();
        let violation = Violation::new(
            agent_id.clone(),
            "test".to_string(),
            "Test violation".to_string(),
        );
        queue.add_violation(violation);
        assert_eq!(queue.pending_violations().len(), 1);
    }
}

