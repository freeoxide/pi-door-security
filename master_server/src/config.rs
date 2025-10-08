use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_bind: String,
    pub token_ttl_hours: i64,
    pub otp_required: bool,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/master".to_string());

        let server_bind = env::var("SERVER_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        let token_ttl_hours = env::var("TOKEN_TTL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(720); // 30 days default

        let otp_required = env::var("OTP_REQUIRED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        Self {
            database_url,
            server_bind,
            token_ttl_hours,
            otp_required,
        }
    }
}
