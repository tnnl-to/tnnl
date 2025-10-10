// Authentication module for verifying Supabase JWT tokens

use anyhow::{anyhow, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // User ID from Supabase
    pub email: String,     // User email
    pub exp: usize,        // Expiration time
    pub iat: usize,        // Issued at
    pub role: String,      // Supabase role (usually "authenticated")
}

pub struct AuthService {
    jwt_secret: String,
}

impl AuthService {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }

    /// Verify a Supabase JWT token and extract claims
    /// Returns (user_id, email) on success
    pub fn verify_supabase_token(&self, token: &str) -> Result<(Uuid, String)> {
        // Decode the JWT header to check algorithm
        let _header = decode_header(token)?;

        // Set up validation
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["authenticated"]);

        // Decode and validate the token
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        ).map_err(|e| anyhow!("Token validation failed: {}", e))?;

        // Extract user ID and email from claims
        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|e| anyhow!("Invalid user ID in token: {}", e))?;

        let email = token_data.claims.email.clone();

        Ok((user_id, email))
    }

    /// Verify token without strict validation (for development/testing)
    /// ONLY use this in development environments
    /// Enable by setting DEV_MODE=true environment variable
    pub fn verify_token_insecure(&self, token: &str) -> Result<(Uuid, String)> {
        // In development, we might want to accept tokens without full validation
        // This should NEVER be used in production
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        validation.required_spec_claims.clear(); // Don't require any specific claims

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret("".as_bytes()),
            &validation,
        ).map_err(|e| anyhow!("Token decode failed: {}", e))?;

        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .unwrap_or_else(|_| Uuid::new_v4());

        let email = token_data.claims.email.clone();

        Ok((user_id, email))
    }
}
