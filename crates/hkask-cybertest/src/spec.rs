use crate::disturbance::Disturbance;

#[derive(Debug, Clone)]
pub struct CyberTestSpec {
    pub policy: &'static str,
    pub context: &'static str,
    pub disturbance: Disturbance,
    pub expectation: CyberExpectation,
}

impl CyberTestSpec {
    pub fn new(
        policy: &'static str,
        context: &'static str,
        disturbance: Disturbance,
        expectation: CyberExpectation,
    ) -> Self {
        Self {
            policy,
            context,
            disturbance,
            expectation,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CyberExpectation {
    pub must_emit_spans: Vec<&'static str>,
    pub must_not_emit_spans: Vec<&'static str>,
    pub variety: Option<VarietyBudget>,
    pub escalation: EscalationExpectation,
}

impl CyberExpectation {
    pub fn with_spans(mut self, spans: Vec<&'static str>) -> Self {
        self.must_emit_spans = spans;
        self
    }

    pub fn without_spans(mut self, spans: Vec<&'static str>) -> Self {
        self.must_not_emit_spans = spans;
        self
    }

    pub fn with_variety(mut self, variety: VarietyBudget) -> Self {
        self.variety = Some(variety);
        self
    }

    pub fn with_escalation(mut self, escalation: EscalationExpectation) -> Self {
        self.escalation = escalation;
        self
    }
}

#[derive(Debug, Clone)]
pub struct VarietyBudget {
    pub absorbed_at_least: i64,
    pub deficit_below: i64,
}

#[derive(Debug, Clone, Default)]
pub enum EscalationExpectation {
    #[default]
    None,
    Required {
        threshold: i64,
    },
}
