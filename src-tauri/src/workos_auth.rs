use serde::{Deserialize, Serialize};

const SUPABASE_URL: &str = "https://phcryrzfeatorjdydenp.supabase.co";
const SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InBoY3J5cnpmZWF0b3JqZHlkZW5wIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTk5MjgzNTIsImV4cCI6MjA3NTUwNDM1Mn0.FqT6HZKFJU4l8DxtWNHpXtZEVCvqKejhC3jwlvxMPmY";

// Development mode: any code "123456" will work with any email
const DEV_MODE_CODE: &str = "123456";

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseOtpResponse {
    // Supabase doesn't return anything meaningful for OTP send, just success/error
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseUser {
    pub id: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseSession {
    pub access_token: String,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseVerifyResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

// For compatibility with existing TypeScript code
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyCodeResponse {
    pub access_token: String,
    pub user: User,
}

pub async fn send_magic_link(email: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/v1/otp", SUPABASE_URL))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "email": email,
            "create_user": true,
            "data": {},
            "options": {
                "should_create_user": true,
                "email_redirect_to": null
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Failed to send OTP ({}): {}", status, response_text));
    }

    Ok(email)
}

pub async fn verify_magic_code(code: String, auth_id: String) -> Result<VerifyCodeResponse, String> {
    println!("[Auth] Verifying OTP code for: {}", auth_id);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/v1/verify", SUPABASE_URL))
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "type": "email",
            "email": auth_id,
            "token": code
        }))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Verification failed ({}): {}", status, response_text));
    }

    let supabase_resp: SupabaseVerifyResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let user = User {
        id: supabase_resp.user.id,
        email: supabase_resp.user.email.unwrap_or_else(|| auth_id.clone()),
    };

    let response = VerifyCodeResponse {
        access_token: supabase_resp.access_token,
        user,
    };

    Ok(response)
}
