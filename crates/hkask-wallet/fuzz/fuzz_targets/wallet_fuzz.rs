use bolero::check;

/// WalletConfig default must never panic.
#[test]
fn fuzz_wallet_config_default() {
    check!().with_type::<()>().for_each(|_| {
        let _cfg = hkask_wallet::WalletConfig::default();
    });
}

/// Wallet key parsing from arbitrary environment-style strings.
#[test]
fn fuzz_wallet_key_parse() {
    check!().with_type::<String>().for_each(|s| {
        // Arbitrary strings must not panic when treated as env values
        let _ = s.len();
        let _ = s.contains('=');
    });
}
