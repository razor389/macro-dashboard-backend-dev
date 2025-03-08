// src/services/google_oauth.rs
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::Result;

/// This matches the format of your JSON service account file
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccountKey {
    pub r#type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

/// The claims needed for Google OAuth
/// `scope` is what we want to access
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,   // issuer: the service account email
    scope: String, // e.g. "https://www.googleapis.com/auth/spreadsheets"
    aud: String,   // audience: "https://oauth2.googleapis.com/token"
    exp: i64,
    iat: i64,
}

/// Load the service account JSON from a file and request a Bearer token
pub async fn fetch_access_token_from_file(
    service_account_json_path: &str,
) -> Result<String> {
    // 1. Read the JSON file
    let json_bytes = std::fs::read(service_account_json_path)?;
    let key: ServiceAccountKey = serde_json::from_slice(&json_bytes)?;

    // 2. Build JWT claims
    let iat = Utc::now();
    let exp = iat + Duration::minutes(59); // token valid ~1 hour
    let claims = Claims {
        iss: key.client_email.clone(),
        scope: "https://www.googleapis.com/auth/spreadsheets".to_string(),
        aud: key.token_uri.clone(),  // typically "https://oauth2.googleapis.com/token"
        exp: exp.timestamp(),
        iat: iat.timestamp(),
    };

    let encoding_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes())?;

    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)?;

    // 4. Exchange the signed JWT for an access token
    #[derive(Debug, Serialize)]
    struct TokenRequest<'a> {
        grant_type: &'a str,
        assertion: &'a str,
    }
    let req_body = TokenRequest {
        grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
        assertion: &jwt,
    };

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct TokenResponse {
        access_token: String,
        token_type: String,
        expires_in: i64,
    }

    let client = Client::new();
    let resp = client
        .post(&key.token_uri)
        .json(&req_body)
        .send()
        .await?
        .error_for_status()?
        .json::<TokenResponse>()
        .await?;

    // 5. Return the actual "access_token"
    Ok(resp.access_token)
}
