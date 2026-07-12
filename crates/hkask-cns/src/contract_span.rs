//! Contract lifecycle CNS spans.
use hkask_types::ObservableSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractSpan {
    ContractProposed,
    ContractAccepted,
    ContractRejected,
    ContractViolated,
}

impl ContractSpan {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractSpan::ContractProposed => "cns.contract.proposed",
            ContractSpan::ContractAccepted => "cns.contract.accepted",
            ContractSpan::ContractRejected => "cns.contract.rejected",
            ContractSpan::ContractViolated => "cns.contract.violated",
        }
    }
}

impl std::fmt::Display for ContractSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ContractSpan {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cns.contract.proposed" => Ok(ContractSpan::ContractProposed),
            "cns.contract.accepted" => Ok(ContractSpan::ContractAccepted),
            "cns.contract.rejected" => Ok(ContractSpan::ContractRejected),
            "cns.contract.violated" => Ok(ContractSpan::ContractViolated),
            _ => Err(()),
        }
    }
}

impl ObservableSpan for ContractSpan {
    fn as_str(&self) -> &'static str {
        ContractSpan::as_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::SpanNamespace;

    #[test]
    fn contract_span_namespaces_are_canonical() {
        let all = vec![
            ContractSpan::ContractProposed,
            ContractSpan::ContractAccepted,
            ContractSpan::ContractRejected,
            ContractSpan::ContractViolated,
        ];
        for span in all {
            let ns = SpanNamespace::new(span.as_str()).unwrap();
            assert_eq!(
                ns.as_str(),
                span.as_str(),
                "ContractSpan::as_str() must match CANONICAL_NAMESPACES"
            );
        }
    }
}
