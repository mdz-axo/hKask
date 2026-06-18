//! Metrics capture and CNS variety — before/after measurement and improvement signal computation.

use super::*;

impl KataEngine {
    fn capture_before_metrics(&self, manifest: &KataManifest, agent: &str, state: &mut KataState) {
        if manifest.metrics.is_empty() {
            return;
        }
        let Some(collector) = self.metric_collector.as_ref() else {
            return;
        };
        let mut metrics = serde_json::Map::new();
        for m in &manifest.metrics {
            if let Some(ref span) = m.span {
                match collector(agent, span) {
                    Ok(value) => {
                        metrics.insert(m.name.clone(), value);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns.kata",
                            metric = %m.name,
                            error = %e,
                            "Failed to capture before metric"
                        );
                    }
                }
            }
        }
        if !metrics.is_empty() {
            state.metric_before = Some(serde_json::Value::Object(metrics));
        }
    }

    fn capture_after_metrics(&self, manifest: &KataManifest, agent: &str, state: &mut KataState) {
        if manifest.metrics.is_empty() {
            return;
        }
        let Some(collector) = self.metric_collector.as_ref() else {
            return;
        };
        let mut metrics = serde_json::Map::new();
        for m in &manifest.metrics {
            if let Some(ref span) = m.span {
                match collector(agent, span) {
                    Ok(value) => {
                        metrics.insert(m.name.clone(), value);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns.kata",
                            metric = %m.name,
                            error = %e,
                            "Failed to capture after metric"
                        );
                    }
                }
            }
        }
        if !metrics.is_empty() {
            state.metric_after = Some(serde_json::Value::Object(metrics));
        }
    }

    fn compute_improvement_signal(&self, state: &KataState) -> Option<ImprovementSignal> {
        let before = state.metric_before.as_ref()?;
        let after = state.metric_after.as_ref()?;

        let delta = match (before, after) {
            (serde_json::Value::Number(b), serde_json::Value::Number(a)) => {
                let bf = b.as_f64()?;
                let af = a.as_f64()?;
                Some(af - bf)
            }
            _ => None,
        };

        let direction = match delta {
            Some(d) if d > 0.0 => ImprovementDirection::Positive,
            Some(d) if d < 0.0 => ImprovementDirection::Negative,
            Some(_) => ImprovementDirection::Stalled,
            None => ImprovementDirection::NotMeasured,
        };

        Some(ImprovementSignal {
            metric_before: Some(before.clone()),
            metric_after: Some(after.clone()),
            delta,
            direction,
        })
    }

    async fn increment_cns_variety(&self, domain: &str, state_name: &str) {
        if let Some(ref cns) = self.cns_runtime {
            cns.read().await.increment_variety(domain, state_name).await;
        }
    }

    async fn check_cns_alerts(&self, manifest: &KataManifest, kata_type: &str) {
        let Some(ref cns) = self.cns_runtime else {
            return;
        };
        let alert = cns
            .read()
            .await
            .check_variety(&manifest.cns.span_namespace)
            .await;
        if let Some(a) = alert {
            tracing::warn!(
                target: "cns.kata",
                namespace = %manifest.cns.span_namespace,
                kata_type = %kata_type,
                severity = ?a.severity,
                deficit = a.deficit,
                threshold = a.threshold,
                "CNS"
            );
        }
    }
}