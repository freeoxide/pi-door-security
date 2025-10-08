use anyhow::Result;
use rand::Rng;
use totp_lite::{totp_custom, Sha1};

const TOTP_STEP: u64 = 30;
const TOTP_DIGITS: u32 = 6;

/// Generate a random OTP secret (base32 encoded)
pub fn generate_otp_secret() -> String {
    let random_bytes: [u8; 20] = rand::thread_rng().gen();
    data_encoding::BASE32_NOPAD.encode(&random_bytes)
}

/// Verify an OTP code against a secret
pub fn verify_otp_code(secret: &str, code: &str) -> Result<bool> {
    let secret_bytes = data_encoding::BASE32_NOPAD.decode(secret.as_bytes())?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Check current time step and one step before/after to account for clock drift
    for time_offset in [-1, 0, 1] {
        let time_step = (now as i64 + (time_offset * TOTP_STEP as i64)) as u64;
        let generated_code = totp_custom::<Sha1>(TOTP_STEP, TOTP_DIGITS, &secret_bytes, time_step);

        if generated_code == code {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generate an otpauth:// URI for authenticator apps
pub fn get_otp_uri(secret: &str, username: &str, issuer: &str) -> String {
    format!(
        "otpauth://totp/{issuer}:{username}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits={TOTP_DIGITS}&period={TOTP_STEP}",
        issuer = urlencoding::encode(issuer),
        username = urlencoding::encode(username),
        secret = secret
    )
}
