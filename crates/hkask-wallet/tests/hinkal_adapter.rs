#![cfg(feature = "hinkal")]
use hkask_types::WebID;
use hkask_types::wallet::{ChainId, WalletError};
use hkask_wallet::hinkal::HinkalPort;
use hkask_wallet::{PrivacyPort, sign_withdrawal};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn set_test_master_key() {
    // SAFETY: test-only env var mutation in isolated test process.
    unsafe {
        std::env::set_var(
            "HKASK_MASTER_KEY",
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
        );
    }
}

// REQ: P9-wlt-hinkal-shielded-withdraw-delta-test — shielded withdraw submits Hinkal /withdraw request and returns tx hash
#[tokio::test]
async fn submit_signed_tx_posts_withdraw_and_returns_tx_hash() {
    set_test_master_key();

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/create-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/withdraw"))
        .and(body_partial_json(serde_json::json!({
            "address": "treasury_pubkey_test",
            "chainId": 501,
            "recipient": "recipient_pubkey",
            "tokenAmounts": [{
                "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "amount": "1500000"
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "txHash": "0xwithdrawhash"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let port = HinkalPort::new(&server.uri(), "treasury_pubkey_test").expect("port");

    let unsigned = port
        .build_unshield_tx("recipient_pubkey", 1_500_000)
        .expect("unsigned payload");

    let actor = WebID::from_persona(b"hinkal-test");
    let tx_hash = port
        .submit_signed_tx(&actor, &unsigned)
        .await
        .expect("withdraw should succeed");

    assert_eq!(tx_hash.0, "0xwithdrawhash");
}

// REQ: P9-wlt-hinkal-shield-payload-test — submit_signed_tx accepts backward-compatible payload+signature envelope
#[tokio::test]
async fn submit_signed_tx_accepts_legacy_payload_plus_signature() {
    set_test_master_key();

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/create-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/withdraw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "txHash": "0xlegacyhash"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let port = HinkalPort::new(&server.uri(), "treasury_pubkey_test").expect("port");

    let unsigned = port
        .build_unshield_tx("recipient_pubkey", 1_500_000)
        .expect("unsigned payload");
    let mut signed = unsigned.clone();
    let sig = sign_withdrawal(ChainId::Hinkal, &unsigned).expect("signature");
    signed.extend_from_slice(&sig);

    let actor = WebID::from_persona(b"hinkal-test");
    let tx_hash = port
        .submit_signed_tx(&actor, &signed)
        .await
        .expect("legacy payload+signature should still be accepted");

    assert_eq!(tx_hash.0, "0xlegacyhash");
}

// REQ: P9-wlt-hinkal-suppress-nonincreasing-test — withdraw fails closed when API omits tx hash in success payload
#[tokio::test]
async fn submit_signed_tx_fails_closed_when_tx_hash_missing() {
    set_test_master_key();

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/create-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/withdraw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let port = HinkalPort::new(&server.uri(), "treasury_pubkey_test").expect("port");

    let unsigned = port
        .build_unshield_tx("recipient_pubkey", 1_500_000)
        .expect("unsigned payload");
    let mut signed = unsigned.clone();
    signed.extend_from_slice(&[7u8; 64]);

    let actor = WebID::from_persona(b"hinkal-test");
    let err = port
        .submit_signed_tx(&actor, &signed)
        .await
        .expect_err("missing tx hash must fail closed");

    match err {
        WalletError::ChainError { chain, message } => {
            assert_eq!(chain, ChainId::Hinkal);
            assert!(message.contains("missing tx hash"));
        }
        other => panic!("expected ChainError, got {other:?}"),
    }
}

// REQ: P9-wlt-hinkal-shield-message-format-test — shielded monitor reports only balance deltas and reuses cached read session
#[tokio::test]
async fn monitor_shielded_transfers_reports_deltas_only() {
    set_test_master_key();

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/create-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "balance": [
                {
                    "token": "USDC",
                    "amount": "5000000",
                    "memo": "dep_ref_abc",
                    "commitment": "commitment_1"
                }
            ]
        })))
        .expect(2)
        .mount(&server)
        .await;

    let port = HinkalPort::new(&server.uri(), "treasury_pubkey_test").expect("port");

    let actor = WebID::from_persona(b"hinkal-monitor-test");
    let first = port
        .monitor_shielded_transfers(&actor)
        .await
        .expect("first poll should succeed");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].amount_usdc_micro, 5_000_000);
    assert_eq!(first[0].memo.as_deref(), Some("dep_ref_abc"));
    assert_eq!(first[0].commitment, "commitment_1");

    let second = port
        .monitor_shielded_transfers(&actor)
        .await
        .expect("second poll should succeed");
    assert!(second.is_empty(), "second poll should only see delta=0");
}
