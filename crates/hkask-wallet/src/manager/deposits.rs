//! Deposit monitoring sub-module.
//!
//! Deposit polling is handled by `WalletManager::start_deposit_monitoring` in the
//! parent module. This file exists to satisfy the `mod deposits;` declaration.
//! Deposit event processing is integrated into the main `WalletManager` impl.

use super::*;
