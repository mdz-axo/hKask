//! Matrix transport builder — Conduit connection and pod registration.

use std::sync::Arc;

pub(crate) async fn build_matrix()
-> Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>> {
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());
    let keychain = hkask_keystore::Keychain::default();

    let credentials = {
        if let Ok(password) = keychain.retrieve_by_key("matrix-bot-curator") {
            Some(("@hkask-curator:localhost".to_string(), password))
        } else if let (Ok(username), Ok(password)) = (
            keychain.retrieve_by_key("matrix-replicant-username"),
            keychain.retrieve_by_key("matrix-replicant-password"),
        ) {
            Some((username, password))
        } else if let (Ok(username), Ok(password)) = (
            std::env::var("HKASK_MATRIX_AGENT_USERNAME"),
            std::env::var("HKASK_MATRIX_AGENT_PASSWORD"),
        ) {
            Some((username, password))
        } else {
            None
        }
    };

    match credentials {
        Some((username, password)) => {
            let mut transport = hkask_communication::matrix::MatrixTransport::new(&homeserver_url);
            match transport.login(&username, &password).await {
                Ok(()) => {
                    let transport = Arc::new(tokio::sync::Mutex::new(transport));
                    let listener =
                        hkask_communication::listener::SevenR7Listener::new(transport.clone(), 30);
                    listener.start().await;
                    tracing::info!(
                        target: "cns.communication.matrix.daemon",
                        username = %username,
                        homeserver = %homeserver_url,
                        "Matrix transport connected and 7R7 listener started"
                    );
                    Some(transport)
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.communication.matrix.daemon",
                        username = %username,
                        error = %e,
                        "Matrix login failed — Conduit may not be running. Continuing without Matrix."
                    );
                    None
                }
            }
        }
        None => {
            tracing::info!(
                target: "cns.communication.matrix.daemon",
                "No Matrix credentials found in keychain or environment. Continuing without Matrix."
            );
            None
        }
    }
}

#[allow(dead_code)]
pub(crate) async fn register_pod_on_matrix(
    homeserver_url: &str,
    _webid: &hkask_types::WebID,
    pod_name: &str,
) {
    let localpart = pod_name.to_lowercase().replace(' ', "-");
    let username = format!("{}-bot", localpart);
    let password = uuid::Uuid::new_v4().to_string();

    let _transport = hkask_communication::matrix::MatrixTransport::new(homeserver_url);

    let url = format!(
        "{}/_matrix/client/v3/register",
        homeserver_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "username": &username,
        "password": &password,
        "initial_device_display_name": format!("hKask Pod: {}", pod_name),
        "auth": {"type": "m.login.dummy"}
    });

    match reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            let full_id = format!("@{}:localhost", username);
            let keychain = hkask_keystore::Keychain::default();
            let _ = keychain.store_by_key(&format!("matrix-pod-{}", pod_name), &password);
            tracing::info!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                matrix_id = %full_id,
                "Pod replicant registered on Matrix"
            );
        }
        Ok(response) => {
            tracing::warn!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                status = %response.status().as_u16(),
                "Matrix registration for pod failed — Conduit may not be running"
            );
        }
        Err(e) => {
            tracing::warn!(
                target: "cns.communication.matrix.pod_registered",
                pod = %pod_name,
                error = %e,
                "Matrix registration for pod failed — Conduit unreachable"
            );
        }
    }
}
