use serde::{Deserialize, Serialize};

const SUPABASE_URL: &str = "https://wohdknhwpjkjlnkkgrot.supabase.co";
const SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6IndvaGRrbmh3cGpramxua2tncm90Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTk4MTExMDYsImV4cCI6MjA3NTM4NzEwNn0.Sx0lq8KY9P7rqTv65WzUUdOvC9MF5JoBDwH7-8CvfCw";

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
    // Always allow dev mode for testing (remove this later for true production)
    println!("[Dev Mode] Bypassing Supabase OTP - use code '{}' to log in", DEV_MODE_CODE);
    return Ok(email);

    // Uncomment for production:
    // let client = reqwest::Client::new();
    // let response = client
    //     .post(format!("{}/auth/v1/otp", SUPABASE_URL))
    //     .header("apikey", SUPABASE_ANON_KEY)
    //     .header("Content-Type", "application/json")
    //     .json(&serde_json::json!({
    //         "email": email,
    //         "create_user": true,
    //         "data": {},
    //         "options": {
    //             "should_create_user": true,
    //             "email_redirect_to": null
    //         }
    //     }))
    //     .send()
    //     .await
    //     .map_err(|e| format!("Network error: {}", e))?;
    // let status = response.status();
    // let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    // if !status.is_success() {
    //     return Err(format!("Failed to send OTP ({}): {}", status, response_text));
    // }
    // Ok(email)
}

pub async fn verify_magic_code(code: String, auth_id: String) -> Result<VerifyCodeResponse, String> {
    // Always allow dev mode for testing (remove this later for true production)
    println!("[Dev Mode] Verifying code: {} for email: {}", code, auth_id);

    if code == DEV_MODE_CODE {

        // Generate a mock JWT token for development
        use uuid::Uuid;
        let user_id = Uuid::new_v4();

        // Create a proper JWT structure (header.payload.signature)
        use base64::engine::Engine;

        // JWT header
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());

        // JWT payload with user claims (including exp for 1 year from now)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let exp = now + (365 * 24 * 60 * 60); // 1 year from now

        let payload_json = format!(
            "{{\"sub\":\"{}\",\"email\":\"{}\",\"iat\":{},\"exp\":{},\"role\":\"authenticated\"}}",
            user_id,
            auth_id,
            now,
            exp
        );
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(payload_json.as_bytes());

        // Mock signature (doesn't matter for insecure validation)
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("dev-mode-signature".as_bytes());

        let mock_token = format!("{}.{}.{}", header, payload, signature);

        let user = User {
            id: user_id.to_string(),
            email: auth_id.clone(),
        };

        println!("[Dev Mode] Login successful for: {}", auth_id);

        return Ok(VerifyCodeResponse {
            access_token: mock_token,
            user,
        });
    }

    // If not dev code, try real Supabase (for future production use)
    // Uncomment for production:
    // let client = reqwest::Client::new();
    // let response = client
    //     .post(format!("{}/auth/v1/verify", SUPABASE_URL))
    //     .header("apikey", SUPABASE_ANON_KEY)
    //     .header("Content-Type", "application/json")
    //     .json(&serde_json::json!({
    //         "type": "signup",
    //         "token": code
    //     }))
    //     .send()
    //     .await
    //     .map_err(|e| format!("Network error: {}", e))?;
    // let status = response.status();
    // let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    // if !status.is_success() {
    //     return Err(format!("Verification failed ({}): {}", status, response_text));
    // }
    // let supabase_resp: SupabaseVerifyResponse = serde_json::from_str(&response_text)
    //     .map_err(|e| format!("JSON parse error: {}", e))?;
    // let user = User {
    //     id: supabase_resp.user.id,
    //     email: supabase_resp.user.email.unwrap_or_else(|| auth_id.clone()),
    // };
    // let response = VerifyCodeResponse {
    //     access_token: supabase_resp.access_token,
    //     user,
    // };
    // Ok(response)

    Err("Invalid code. Use '123456' for development mode.".to_string())
}
