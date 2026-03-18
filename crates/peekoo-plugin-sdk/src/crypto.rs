//! Cryptography helpers for plugins.
//!
//! Requires the `crypto:ed25519` permission.
//! Keys are persisted per plugin and namespaced by alias.

use extism_pdk::{Error, Json};

use crate::host_fns::{
    peekoo_crypto_ed25519_get_or_create, peekoo_crypto_ed25519_sign,
    CryptoEd25519GetOrCreateRequest, CryptoEd25519SignRequest,
};

#[derive(Clone, Debug)]
pub struct Ed25519PublicKeyInfo {
    /// Public key encoded as URL-safe base64 without padding.
    pub public_key_base64_url: String,
    /// SHA-256 digest of the public key in lowercase hex.
    pub public_key_sha256_hex: String,
}

/// Loads or creates a plugin-scoped Ed25519 keypair for the provided alias.
pub fn ed25519_get_or_create(alias: &str) -> Result<Ed25519PublicKeyInfo, Error> {
    let response = unsafe {
        peekoo_crypto_ed25519_get_or_create(Json(CryptoEd25519GetOrCreateRequest {
            alias: alias.to_string(),
        }))?
    };

    Ok(Ed25519PublicKeyInfo {
        public_key_base64_url: response.0.public_key_base64_url,
        public_key_sha256_hex: response.0.public_key_sha256_hex,
    })
}

/// Signs a UTF-8 payload with the plugin-scoped Ed25519 key for `alias`.
pub fn ed25519_sign(alias: &str, payload: &str) -> Result<String, Error> {
    let response = unsafe {
        peekoo_crypto_ed25519_sign(Json(CryptoEd25519SignRequest {
            alias: alias.to_string(),
            payload: payload.to_string(),
        }))?
    };

    Ok(response.0.signature_base64_url)
}
