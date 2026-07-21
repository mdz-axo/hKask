//! Architecture seam CNS spans.
use hkask_types::ObservableSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeamSpan {
    ArchitectureSeamCoverage,
    ArchitectureSeamDrift,
}

impl SeamSpan {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeamSpan::ArchitectureSeamCoverage => "cns.architecture.seam.coverage",
            SeamSpan::ArchitectureSeamDrift => "cns.architecture.seam.drift",
        }
    }
}

impl std::fmt::Display for SeamSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SeamSpan {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cns.architecture.seam.coverage" => Ok(SeamSpan::ArchitectureSeamCoverage),
            "cns.architecture.seam.drift" => Ok(SeamSpan::ArchitectureSeamDrift),
            _ => Err(()),
        }
    }
}

impl ObservableSpan for SeamSpan {
    fn as_str(&self) -> &'static str {
        SeamSpan::as_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::SpanNamespace;

    #[test]
    fn seam_span_namespaces_are_canonical() {
        let all = vec![
            SeamSpan::ArchitectureSeamCoverage,
            SeamSpan::ArchitectureSeamDrift,
        ];
        for span in all {
            let ns = SpanNamespace::new(span.as_str()).unwrap();
            assert_eq!(
                ns.as_str(),
                span.as_str(),
                "SeamSpan::as_str() must match CANONICAL_NAMESPACES"
            );
        }
    }
}
