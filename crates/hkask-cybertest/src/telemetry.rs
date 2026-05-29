use hkask_types::{NuEvent, NuEventSink};
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CapturedEvent {
    pub span: String,
    pub phase: String,
    pub observation: Value,
}

#[derive(Clone, Default)]
pub struct TelemetryCapture {
    inner: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl TelemetryCapture {
    pub fn record(&self, event: CapturedEvent) {
        self.inner.lock().push(event);
    }

    pub fn events(&self) -> Vec<CapturedEvent> {
        self.inner.lock().clone()
    }

    pub fn spans(&self) -> Vec<String> {
        self.inner
            .lock()
            .iter()
            .map(|e| e.span.clone())
            .collect::<Vec<_>>()
    }

    pub fn contains_span_prefix(&self, prefix: &str) -> bool {
        self.inner.lock().iter().any(|e| e.span.starts_with(prefix))
    }
}

pub struct CaptureSink {
    capture: TelemetryCapture,
}

impl CaptureSink {
    pub fn new(capture: TelemetryCapture) -> Self {
        Self { capture }
    }
}

impl NuEventSink for CaptureSink {
    fn persist(&self, event: &NuEvent) -> Result<(), hkask_types::NuEventSinkError> {
        self.capture.record(CapturedEvent {
            span: format!("{:?}", event.span),
            phase: format!("{:?}", event.phase).to_lowercase(),
            observation: event.observation.clone(),
        });
        Ok(())
    }
}
