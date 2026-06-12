//! Telnyx setup helpers — used during onboarding to provision phone numbers
//! and WhatsApp for newly created replicants.
//!
//! These are direct API calls (not MCP tools) so the CLI onboarding can
//! provision comms without spawning the full MCP server.

use crate::error::ServiceError;

const BASE_URL: &str = "https://api.telnyx.com/v2";

/// Build an authenticated reqwest client for Telnyx API v2.
fn telnyx_client(api_key: &str) -> Result<reqwest::Client, ServiceError> {
    let mut headers = reqwest::header::HeaderMap::new();
    let auth = format!("Bearer {api_key}");
    let val = reqwest::header::HeaderValue::from_str(&auth)
        .map_err(|e| ServiceError::Config(format!("Invalid API key format: {e}")))?;
    headers.insert(reqwest::header::AUTHORIZATION, val);
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| ServiceError::Config(format!("Failed to build HTTP client: {e}")))
}

/// Verify the Telnyx API key works by fetching the first page of phone numbers.
pub async fn verify_api_key(api_key: &str) -> Result<bool, ServiceError> {
    let client = telnyx_client(api_key)?;
    let resp = client
        .get(format!("{BASE_URL}/phone_numbers?page_size=1"))
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Telnyx API unreachable: {e}")))?;
    Ok(resp.status().is_success())
}

/// Search available phone numbers. Returns a list of phone numbers in E.164 format.
/// If `area_code` is provided, filters to that NPA.
/// If `contains` is provided, filters to numbers containing that digit sequence.
pub async fn search_available_numbers(
    api_key: &str,
    area_code: Option<&str>,
    contains: Option<&str>,
) -> Result<Vec<String>, ServiceError> {
    let client = telnyx_client(api_key)?;
    let mut url = format!(
        "{BASE_URL}/available_phone_numbers?filter[voice]=true&filter[sms]=true&page_size=10"
    );
    if let Some(code) = area_code {
        url.push_str(&format!("&filter[npa]={code}"));
    }
    if let Some(pattern) = contains {
        url.push_str(&format!("&filter[phone_number][contains]={pattern}"));
    }
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to search numbers: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Config(format!(
            "Number search failed: {body}"
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to parse response: {e}")))?;

    let numbers: Vec<String> = json["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|entry| entry["phone_number"].as_str().map(|s| s.to_string()))
        .collect();

    Ok(numbers)
}

/// Order (buy) a phone number. Requires a messaging profile ID.
/// Returns the ordered phone number on success.
pub async fn order_number(
    api_key: &str,
    phone_number: &str,
    messaging_profile_id: &str,
) -> Result<String, ServiceError> {
    let client = telnyx_client(api_key)?;
    let payload = serde_json::json!({
        "phone_numbers": [{"phone_number": phone_number}],
        "messaging_profile_id": messaging_profile_id,
    });

    let resp = client
        .post(format!("{BASE_URL}/number_orders"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to order number: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Config(format!("Number order failed: {body}")));
    }

    Ok(phone_number.to_string())
}

/// Get or create a messaging profile for SMS/WhatsApp.
/// Returns the messaging profile ID.
pub async fn get_messaging_profile(api_key: &str) -> Result<String, ServiceError> {
    let client = telnyx_client(api_key)?;

    // Try to list existing profiles first
    let resp = client
        .get(format!("{BASE_URL}/messaging_profiles?page_size=1"))
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to list profiles: {e}")))?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if let Some(first) = json["data"].as_array().and_then(|a| a.first())
            && let Some(id) = first["id"].as_str()
        {
            return Ok(id.to_string());
        }
    }

    // No existing profile — create one
    let payload = serde_json::json!({
        "name": "hKask Replicant Messaging",
        "enabled": true,
    });
    let resp = client
        .post(format!("{BASE_URL}/messaging_profiles"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to create profile: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Config(format!(
            "Profile creation failed: {body}"
        )));
    }

    let json: serde_json::Value = resp.json().await.unwrap_or_default();
    json["data"]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ServiceError::Config("No profile ID in response".to_string()))
}

/// Send the welcome SMS — the replicant introduces itself to the human user.
pub async fn send_welcome_sms(
    api_key: &str,
    from: &str, // replicant's new phone number
    to: &str,   // human user's phone
    replicant_name: &str,
) -> Result<(), ServiceError> {
    let client = telnyx_client(api_key)?;
    let message = format!(
        "Hi! I'm {replicant_name}, your hKask replicant. You can reach me at {from} via SMS or WhatsApp. Save this number!"
    );
    let payload = serde_json::json!({
        "from": from,
        "to": to,
        "text": message,
    });

    let resp = client
        .post(format!("{BASE_URL}/messages"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to send SMS: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Config(format!("SMS send failed: {body}")));
    }

    Ok(())
}

/// Generate TTS audio from text using a Telnyx voice.
/// Returns the path to a WAV file saved in the system temp directory.
pub async fn tts_generate(
    api_key: &str,
    text: &str,
    voice_id: &str,
) -> Result<String, ServiceError> {
    let client = telnyx_client(api_key)?;
    let payload = serde_json::json!({
        "text": text,
        "voice_id": voice_id,
        "format": "wav",
    });

    let resp = client
        .post(format!("{BASE_URL}/tts"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ServiceError::Config(format!("TTS API unreachable: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ServiceError::Config(format!(
            "TTS generation failed: {body}"
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ServiceError::Config(format!("Failed to read audio: {e}")))?;

    let file_name = format!("hkask-tts-{}.wav", uuid::Uuid::new_v4());
    let path = std::env::temp_dir().join(&file_name);
    std::fs::write(&path, &bytes)
        .map_err(|e| ServiceError::Config(format!("Failed to save audio: {e}")))?;

    Ok(path.to_string_lossy().to_string())
}
