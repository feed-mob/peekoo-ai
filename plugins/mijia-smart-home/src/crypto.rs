use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use peekoo_plugin_sdk::prelude::*;
use rc4::{KeyInit, Rc4, StreamCipher};
use sha1::Sha1;
use sha2::Sha256;

use crate::error::MijiaError;

pub fn gen_nonce() -> String {
    let millis = peekoo::system::time_millis().unwrap_or(0);
    let mut bytes = Vec::with_capacity(16);

    // 8 random bytes (signed int64)
    let uuid = peekoo::system::uuid_v4().unwrap_or_default();
    let rand_val: i64 = (uuid
        .as_bytes()
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, b)| acc | ((*b as u64) << (i * 8)))
        & 0x7fff_ffff_ffff_ffff) as i64;
    bytes.extend_from_slice(&rand_val.to_be_bytes());

    // floor(millis / 60000)
    let part2 = millis / 60000;
    let part2_bytes = part2.to_be_bytes();
    let start = part2_bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes.extend_from_slice(&part2_bytes[start..]);

    B64.encode(&bytes)
}

pub fn get_signed_nonce(ssecurity: &str, nonce: &str) -> Result<String, MijiaError> {
    let sec_bytes = B64
        .decode(ssecurity)
        .map_err(|e| MijiaError::Crypto(format!("ssecurity decode: {e}")))?;
    let nonce_bytes = B64
        .decode(nonce)
        .map_err(|e| MijiaError::Crypto(format!("nonce decode: {e}")))?;

    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(&sec_bytes);
    hasher.update(&nonce_bytes);
    let digest = hasher.finalize();

    Ok(B64.encode(digest))
}

pub fn gen_enc_signature(
    uri: &str,
    method: &str,
    signed_nonce: &str,
    params: &std::collections::BTreeMap<String, String>,
) -> Result<String, MijiaError> {
    let mut parts = vec![method.to_uppercase(), uri.to_string()];
    for (k, v) in params {
        parts.push(format!("{k}={v}"));
    }
    parts.push(signed_nonce.to_string());
    let signature_string = parts.join("&");

    use sha1::Digest;
    let mut hasher = Sha1::new();
    hasher.update(signature_string.as_bytes());
    let digest = hasher.finalize();

    Ok(B64.encode(digest))
}

fn rc4_transform(key_b64: &str, data: &[u8]) -> Result<Vec<u8>, MijiaError> {
    let key = B64
        .decode(key_b64)
        .map_err(|e| MijiaError::Crypto(format!("RC4 key decode: {e}")))?;

    let mut cipher =
        Rc4::new_from_slice(&key).map_err(|e| MijiaError::Crypto(format!("RC4 init: {e}")))?;

    // Discard first 1024 bytes (standard Mi RC4 convention)
    let mut discard = vec![0u8; 1024];
    cipher.apply_keystream(&mut discard);

    let mut buf = data.to_vec();
    cipher.apply_keystream(&mut buf);
    Ok(buf)
}

pub fn encrypt_rc4(key_b64: &str, plaintext: &str) -> Result<String, MijiaError> {
    let encrypted = rc4_transform(key_b64, plaintext.as_bytes())?;
    Ok(B64.encode(encrypted))
}

pub fn decrypt_rc4(key_b64: &str, payload_b64: &str) -> Result<Vec<u8>, MijiaError> {
    let encrypted = B64
        .decode(payload_b64)
        .map_err(|e| MijiaError::Crypto(format!("decrypt input decode: {e}")))?;
    rc4_transform(key_b64, &encrypted)
}

pub fn decrypt_response(ssecurity: &str, nonce: &str, payload: &str) -> Result<String, MijiaError> {
    let signed_nonce = get_signed_nonce(ssecurity, nonce)?;
    let decrypted = decrypt_rc4(&signed_nonce, payload)?;

    // Try UTF-8 first, then gzip
    match String::from_utf8(decrypted.clone()) {
        Ok(s) => Ok(s),
        Err(_) => {
            use flate2::read::GzDecoder;
            use std::io::Read;
            let mut decoder = GzDecoder::new(&decrypted[..]);
            let mut decompressed = String::new();
            decoder
                .read_to_string(&mut decompressed)
                .map_err(|e| MijiaError::Crypto(format!("gzip decompress: {e}")))?;
            Ok(decompressed)
        }
    }
}

pub fn build_enc_params(
    uri: &str,
    method: &str,
    signed_nonce: &str,
    nonce: &str,
    ssecurity: &str,
    data_json: &str,
) -> Result<std::collections::BTreeMap<String, String>, MijiaError> {
    let mut params = std::collections::BTreeMap::new();
    params.insert("data".to_string(), data_json.to_string());

    // Compute rc4_hash__ over plaintext params
    let rc4_hash = gen_enc_signature(uri, method, signed_nonce, &params)?;

    // RC4-encrypt each param value
    for v in params.values_mut() {
        *v = encrypt_rc4(signed_nonce, v)?;
    }
    params.insert("rc4_hash__".to_string(), rc4_hash);

    // Compute final signature over encrypted params
    let signature = gen_enc_signature(uri, method, signed_nonce, &params)?;
    params.insert("signature".to_string(), signature);
    params.insert("ssecurity".to_string(), ssecurity.to_string());
    params.insert("_nonce".to_string(), nonce.to_string());

    Ok(params)
}

pub fn encode_form_params(params: &std::collections::BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push(hex_digit(byte >> 4));
                result.push(hex_digit(byte & 0xf));
            }
        }
    }
    result
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        _ => unreachable!(),
    }
}
