//! Test Fixture Port - Inbound port for test data generation
//!
//! This port provides standardized fixture builders for hKask test infrastructure.
//! Each fixture builder implements the corresponding production port trait.

use chrono::{DateTime, Utc};
use hkask_types::{NuEvent, Phase, Span, TemplateType, WebID};
use serde_json::Value;

/// Fixture builder for WebID entities
pub struct WebIDFixture {
    webid: WebID,
}

impl WebIDFixture {
    pub fn new() -> Self {
        Self {
            webid: WebID::new(),
        }
    }

    pub fn build(&self) -> WebID {
        self.webid
    }

    pub fn with_webid(mut self, webid: WebID) -> Self {
        self.webid = webid;
        self
    }
}

impl Default for WebIDFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture builder for NuEvent entities
pub struct NuEventFixture {
    observer_webid: WebID,
    span: Span,
    phase: Phase,
    observation: Value,
    recursion_depth: u8,
}

impl NuEventFixture {
    pub fn new() -> Self {
        Self {
            observer_webid: WebID::new(),
            span: Span::prompt("test"),
            phase: Phase::Observe,
            observation: Value::Null,
            recursion_depth: 0,
        }
    }

    pub fn build(&self) -> NuEvent {
        NuEvent::new(
            self.observer_webid,
            self.span.clone(),
            self.phase,
            self.observation.clone(),
            self.recursion_depth,
        )
    }

    pub fn with_observer(mut self, webid: WebID) -> Self {
        self.observer_webid = webid;
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn with_phase(mut self, phase: Phase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_observation(mut self, observation: Value) -> Self {
        self.observation = observation;
        self
    }

    pub fn with_recursion_depth(mut self, depth: u8) -> Self {
        self.recursion_depth = depth;
        self
    }
}

impl Default for NuEventFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture builder for template entities
pub struct TemplateFixture {
    template_type: TemplateType,
    domain: String,
    lexicon_terms: Vec<String>,
}

impl TemplateFixture {
    pub fn new() -> Self {
        Self {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            lexicon_terms: vec![],
        }
    }

    pub fn build(&self) -> (TemplateType, String, Vec<String>) {
        (
            self.template_type,
            self.domain.clone(),
            self.lexicon_terms.clone(),
        )
    }

    pub fn with_type(mut self, template_type: TemplateType) -> Self {
        self.template_type = template_type;
        self
    }

    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }

    pub fn with_lexicon_terms(mut self, terms: Vec<String>) -> Self {
        self.lexicon_terms = terms;
        self
    }
}

impl Default for TemplateFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture builder for timestamps
pub struct TimestampFixture {
    timestamp: Option<DateTime<Utc>>,
}

impl TimestampFixture {
    pub fn new() -> Self {
        Self { timestamp: None }
    }

    pub fn build(&self) -> DateTime<Utc> {
        self.timestamp.unwrap_or_else(Utc::now)
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn fixed() -> Self {
        Self {
            timestamp: Some(DateTime::from_timestamp(0, 0).unwrap_or_else(Utc::now)),
        }
    }
}

impl Default for TimestampFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Composite fixture builder for complex test scenarios
pub struct TestFixture {
    webid: WebIDFixture,
    event: NuEventFixture,
    template: TemplateFixture,
    timestamp: TimestampFixture,
}

impl TestFixture {
    pub fn new() -> Self {
        Self {
            webid: WebIDFixture::new(),
            event: NuEventFixture::new(),
            template: TemplateFixture::new(),
            timestamp: TimestampFixture::new(),
        }
    }

    pub fn webid(&self) -> &WebIDFixture {
        &self.webid
    }

    pub fn event(&self) -> &NuEventFixture {
        &self.event
    }

    pub fn template(&self) -> &TemplateFixture {
        &self.template
    }

    pub fn timestamp(&self) -> &TimestampFixture {
        &self.timestamp
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}
