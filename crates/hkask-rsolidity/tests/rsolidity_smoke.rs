//! Smoke tests for the rSolidity macro vocabulary.

use hkask_rsolidity::{self as rs, Ocap};
use hkask_types::cns::CnsSpan;
use hkask_types::event::Phase;
use serde_json::json;

// REQ: P9-rsolidity-ocap-gate — OCAP attribute delegates to receiver trait
#[derive(Debug, PartialEq)]
struct OcapError(&'static str);

struct Vault;

impl Ocap for Vault {
    type Error = OcapError;
    fn verify_ocap(&self, resource: &str, operation: &str) -> Result<(), Self::Error> {
        if resource == "wallet_balance" && operation == "debit" {
            Ok(())
        } else {
            Err(OcapError("denied"))
        }
    }
}

impl Vault {
    #[rs::ocap(resource = "wallet_balance", operation = "debit")]
    fn debit(&self, amount: u64) -> Result<u64, OcapError> {
        Ok(amount)
    }

    #[rs::ocap(resource = "treasury", operation = "drain")]
    fn drain(&self) -> Result<u64, OcapError> {
        Ok(0)
    }
}

// REQ: P9-rsolidity-contract-attribute — contract metadata validates id/principle
#[rs::contract(
    id = "P9-test-contract",
    principle = "P9",
    pre = "input is valid",
    post = "output equals input"
)]
#[allow(dead_code)]
fn identity(x: u64) -> u64 {
    x
}

#[test]
fn ocap_allows_authorized_operation() {
    assert_eq!(Vault.debit(10).unwrap(), 10);
}

#[test]
fn ocap_denies_unauthorized_operation() {
    assert!(Vault.drain().is_err());
}

#[test]
fn require_passes_on_true() {
    rs::require!(true, "P9-test", "should not panic");
}

#[test]
#[should_panic(expected = "require violated [P9-test]:")]
fn require_panics_on_false() {
    rs::require!(false, "P9-test", "expected panic");
}

#[test]
fn assert_passes_on_true() {
    rs::assert!(1 + 1 == 2, "P9-test", "should not panic");
}

#[test]
#[should_panic(expected = "assert violated [P9-test]:")]
fn assert_panics_on_false() {
    rs::assert!(1 + 1 == 3, "P9-test", "expected panic");
}

#[test]
fn revert_returns_error() {
    fn inner() -> Result<u64, &'static str> {
        rs::revert!("P9-test", "boom");
    }
    assert_eq!(inner().unwrap_err(), "boom");
}

#[test]
fn emit_does_not_panic() {
    rs::emit!(
        CnsSpan::WalletWithdrawal,
        "submitted",
        Phase::Act,
        json!({ "actor": "did:web:alice", "tx_hash": "0x01" })
    );
}

#[test]
fn contract_attribute_compiles() {
    assert_eq!(identity(7), 7);
}
