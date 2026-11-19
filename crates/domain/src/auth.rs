//! Пароли (argon2id) и JWT HS256. Договорът от ytbuch: 7 дни TTL,
//! issuer "dzambo", `ver` на сесията за инвалидиране при смяна на парола.

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::{Argon2, PasswordHasher};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};

pub const ISSUER: &str = "dzambo";
pub const TOKEN_TTL_SECS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
    pub iss: String,
    #[serde(default)]
    pub ver: i32,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = argon2::password_hash::SaltString::generate(&mut rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Връща false и за невалиден PHC низ — грешката не тече навън.
pub fn verify_password(password: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn issue_token(secret: &str, sub: &str, ver: i32) -> Result<(String, u64), String> {
    let iat = now_secs();
    let exp = iat + TOKEN_TTL_SECS;
    let claims = Claims {
        sub: sub.to_string(),
        iat,
        exp,
        iss: ISSUER.to_string(),
        ver,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map(|token| (token, TOKEN_TTL_SECS))
    .map_err(|e| e.to_string())
}

pub fn verify_token(secret: &str, token: &str) -> Option<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[ISSUER]);
    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    ) {
        Ok(data) => Some(data.claims),
        Err(e) if matches!(e.kind(), ErrorKind::ExpiredSignature) => None,
        Err(_) => None,
    }
}

/// Случайен hex низ — за генерирана парола и JWT тайни.
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(s.as_bytes()))
}
