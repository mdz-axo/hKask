//! Budget, gas, and fee operations.

use super::*;


impl WalletManager {
    pub async fn estimate_withdrawal_fee(
        &self,
        actor: &WebID,
        chain: ChainId,
    ) -> Result<WithdrawalFee, WalletError> {
        let rate = self.price_feed.get_rate(chain).await.inspect_err(|e| {
            self.emit_chain_error_for_actor(
                actor,
                chain,
                "estimate_withdrawal_fee",
                &e.to_string(),
            );
        })?;
        Ok(estimate_withdrawal_fee(
            chain,
            &rate,
            self.config.rj_per_usdc,
        ))
    }

    /// Convert micro-USDC to rJoules.
    fn usdc_to_rjoules(&self, usdc_micro: u64) -> RJoule {
        let rj = (usdc_micro as u128 * self.config.rj_per_usdc as u128 / 1_000_000) as u64;
        RJoule::new(rj)
    }

    /// Convert rJoules to micro-USDC.
    fn rjoules_to_usdc(&self, rj: RJoule) -> u64 {
        (rj.as_u64() as u128 * 1_000_000 / self.config.rj_per_usdc as u128) as u64
    }

    /// Check if a wallet can afford a given rJoule cost.
    ///
    /// REQ: P9-wallet-mgr-can-afford
    /// \[P9\] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend
    /// \[P4\] Constraining: Clear Boundaries — cannot reserve beyond balance
    /// pre:  wallet_id is a valid WalletId, cost_rj is a valid RJoule
    /// post: returns Ok(true) iff balance.rjoules >= cost_rj
    /// post: returns Ok(false) iff balance.rjoules < cost_rj
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, WalletError> {
        let balance = self.get_balance(wallet_id)?;
        Ok(balance.rjoules >= cost_rj.as_u64())
    }

    /// Reserve rJoules for an in-flight operation (optimistic).
    /// The actual debit happens at settle time.
    ///
    /// REQ: P9-wallet-mgr-reserve
    /// \[P9\] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend
    /// \[P4\] Constraining: Clear Boundaries — cannot reserve beyond balance
    /// pre:  wallet_id is a valid WalletId, amount is a valid RJoule
    /// post: if can_afford → Ok(()), reservation is optimistic (no debit)
    /// post: if !can_afford → Err(InsufficientBalance)
    pub fn reserve_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<(), WalletError> {
        if !self.can_afford(wallet_id, amount)? {
            let balance = self.get_balance(wallet_id)?;
            return Err(WalletError::InsufficientBalance {
                have: RJoule::new(balance.rjoules),
                need: amount,
            });
        }
        // Reservation is optimistic — we check can_afford but don't debit yet.
        // The actual debit happens in settle_rjoules.
        Ok(())
    }

    /// Settle rJoules after an operation completes.
    /// Debits the actual cost (may be less than reserved on failure).
    ///
    /// REQ: P9-wallet-mgr-settle
    /// \[P9\] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend
    /// \[P4\] Constraining: Clear Boundaries — cannot reserve beyond balance
    /// pre:  wallet_id is a valid WalletId, reserved and actual are valid RJoule
    /// post: wallet balance debited by actual (not reserved)
    /// post: if actual < reserved, difference is implicitly refunded
    pub fn settle_rjoules(
        &self,
        wallet_id: WalletId,
        reserved: RJoule,
        actual: RJoule,
    ) -> Result<(), WalletError> {
        self.store.debit_rjoules(wallet_id, actual)?;
        // If actual < reserved, the difference is implicitly refunded
        // (we only debit actual, not reserved).
        let _ = reserved; // reserved amount is informational
        Ok(())
    }

    // ── Deposit reference scheme (merged from deposit_ref.rs) ─────────────────

    /// Generate a one-time deposit reference for shielded deposits.
    ///
    /// # Privacy property `[IS-DECL]`
    /// Derived via HKDF from the wallet seed + nonce + expiry.
    /// Appears random on-chain but hKask can verify it belongs to a specific wallet.
    ///
    /// # Anti-replay `[OUGHT-DECL]`
    /// References are burned on use (consumed in WalletStore).
    /// References expire after `validity_duration` (default 24h).
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        validity_duration: Duration,
    ) -> Result<DepositReference, WalletError> {
        let nonce: [u8; 16] = rand::random();
        let expiry = Utc::now() + validity_duration;
        // REQ: P9-wallet-mgr-deposit-ref-nonce — HKDF context includes nonce to bind reference to its specific random nonce
        let context = format!(
            "hkask:deposit-ref:{}:{}:{}:{}",
            wallet_id,
            chain,
            expiry.timestamp(),
            hex::encode(nonce)
        );
        // HKDF-expand from wallet seed
        let ref_bytes = hkdf_expand(&*self.wallet_seed, context.as_bytes())?;
        let reference = hex::encode(&ref_bytes[..16]); // 32-char hex string

        let dep_ref = DepositReference {
            reference,
            wallet_id,
            chain,
            nonce,
            expires_at: expiry,
        };
        self.store.store_deposit_reference(&dep_ref)?;
        Ok(dep_ref)
    }

    // ── Encumbrance — rJoule lock/release/consume ────────────────────────────

    /// Encumber rJoules from a wallet for an API key's allocation.
    ///
    /// REQ: P9-wallet-mgr-encumber
    /// \[P9\] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys
    /// \[P4\] Constraining: Clear Boundaries — only the entitled key can consume
    /// \[P8\] Constraining: Semantic Grounding — atomic consume/release preserves balance
    /// pre:  wallet_id is a valid WalletId, key_id is a valid ApiKeyId, amount > 0
    /// post: amount rJoules locked against wallet for key_id
    /// post: emits cns.wallet.encumbered span if event_sink configured
    /// Locks `amount` rJoules against the wallet balance. The locked rJoules
    /// can only be consumed by the specified API key via `consume()`.
    /// Unspent rJoules are returned to the wallet on `release_encumbrance()`.
    pub fn encumber(
        &self,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        amount: RJoule,
    ) -> Result<(), WalletError> {
        self.store.encumber_rjoules(wallet_id, key_id, amount)?;
        self.emit_span(
            CnsSpan::Gas,
            "encumbered",
            Phase::Act,
            serde_json::json!({
                "key_id": key_id.to_string(),
                "wallet_id": wallet_id.to_string(),
                "amount_rj": amount.as_u64(),
            }),
        );
        Ok(())
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    ///
    /// REQ: P9-wallet-mgr-release-encumbrance
    /// \[P9\] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys
    /// \[P4\] Constraining: Clear Boundaries — only the entitled key can consume
    /// \[P8\] Constraining: Semantic Grounding — atomic consume/release preserves balance
    /// pre:  key_id is a valid ApiKeyId
    /// post: unspent rJoules returned to wallet
    /// post: idempotent — releasing already-released/consumed encumbrance is no-op
    /// Idempotent — releasing an already-released or consumed encumbrance
    /// is a no-op.
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        self.store.release_encumbrance(key_id)?;
        self.emit_span(
            CnsSpan::Gas,
            "released",
            Phase::Act,
            serde_json::json!({
                "key_id": key_id.to_string(),
            }),
        );
        Ok(())
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    ///
    /// REQ: P9-wallet-mgr-consume
    /// \[P9\] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys
    /// \[P4\] Constraining: Clear Boundaries — only the entitled key can consume
    /// \[P8\] Constraining: Semantic Grounding — atomic consume/release preserves balance
    /// pre:  key_id is a valid ApiKeyId, gas_rj > 0
    /// post: gas_rj deducted from key's active encumbrance (atomic)
    /// post: if encumbrance fully consumed → status transitions to 'consumed'
    /// Deducts `gas_rj` from the key's active encumbrance. This is a single
    /// atomic operation — no separate check+deduct pair. If the encumbrance
    /// is fully consumed, status transitions to 'consumed'.
    pub fn consume(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), WalletError> {
        self.store.consume_encumbrance(key_id, gas_rj)?;
        Ok(())
    }

    /// Get the encumbrance for an API key.
    ///
    /// REQ: P9-wallet-mgr-get-encumbrance
    /// \[P9\] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys
    /// \[P4\] Constraining: Clear Boundaries — only the entitled key can consume
    /// \[P8\] Constraining: Semantic Grounding — atomic consume/release preserves balance
    /// pre:  key_id is a valid ApiKeyId
    /// post: returns Ok(Some(encumbrance)) if key has active encumbrance
    /// post: returns Ok(None) if key has no encumbrance
    pub fn get_encumbrance(&self, key_id: ApiKeyId) -> Result<Option<Encumbrance>, WalletError> {
        self.store.get_encumbrance(key_id)
    }
}

// ── HKDF helper (minimal, uses hmac + sha2 from workspace) ─────────────────────

fn hkdf_expand(seed: &[u8], info: &[u8]) -> Result<Vec<u8>, WalletError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(seed).map_err(|e| {
        WalletError::Infra(hkask_types::InfrastructureError::Database(e.to_string()))
    })?;
    mac.update(info);
    mac.update(&[0x01]);
    let result = mac.finalize().into_bytes();
    Ok(result.to_vec())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
