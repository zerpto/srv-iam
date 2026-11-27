use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256. Emailed tokens are stored only in this form.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
