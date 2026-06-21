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
