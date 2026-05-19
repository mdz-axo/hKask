//! CNS span emission

use hkask_types::{NuEvent, Span, WebID};
use serde_json::Value;
use tracing::info;

/// CNS span emitter
pub struct SpanEmitter {
    observer_webid: WebID,
}

impl SpanEmitter {
    pub fn new(observer_webid: WebID) -> Self {
        Self { observer_webid }
    }

    pub fn emit(&self, span: Span, phase: hkask_types::Phase, observation: Value) {
        let event = NuEvent::new(self.observer_webid.clone(), span, phase, observation, 0);

        info!(target: "cns", event = ?event.id, span = ?event.span, "CNS event emitted");
    }
}
