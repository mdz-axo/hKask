//! Smoke tests for the rSolidity macro vocabulary.

use hkask_rsolidity::{self as rs, Ocap};
use serde::Serialize;

#[derive(Debug, PartialEq)]
struct OcapError(&'static str);

struct Vault;

#[derive(Debug, Serialize)]
#[allow(dead_code)]
enum Phase {
    Act,
}

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

