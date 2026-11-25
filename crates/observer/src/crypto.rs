use aes::Aes128;
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::{Digest, Sha256};

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;
type HmacSha256 = Hmac<Sha256>;

/// Stable salt so one `SECRET_KEY` keeps decrypting secrets written by the Python build.
const KDF_SALT: &[u8] = b"observer-lite-salt";
const KDF_ITERS: u32 = 100_000;

#[derive(Debug)]
pub struct CryptoError;

pub fn derive_fernet_key(secret: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(secret.as_bytes(), KDF_SALT, KDF_ITERS, &mut key);
    key
}

/// Fernet token (AES-128-CBC + HMAC-SHA256), compatible with Python `cryptography.fernet`.
pub fn fernet_encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<String, CryptoError> {
    let signing = &key[..16];
    let encryption = &key[16..];
    let mut iv = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut iv);
    let timestamp = chrono::Utc::now().timestamp().max(0) as u64;

    let mut buf = vec![0u8; plaintext.len() + 16];
    buf[..plaintext.len()].copy_from_slice(plaintext);
    let ciphertext = Aes128CbcEnc::new(encryption.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext.len())
        .map_err(|_| CryptoError)?
        .to_vec();

    let mut basic = Vec::with_capacity(1 + 8 + 16 + ciphertext.len());
    basic.push(0x80);
    basic.extend_from_slice(&timestamp.to_be_bytes());
    basic.extend_from_slice(&iv);
    basic.extend_from_slice(&ciphertext);

    let mut mac = HmacSha256::new_from_slice(signing).map_err(|_| CryptoError)?;
    mac.update(&basic);
    let sig = mac.finalize().into_bytes();
    basic.extend_from_slice(&sig);
    Ok(URL_SAFE.encode(basic))
}

pub fn fernet_decrypt(key: &[u8; 32], token: &str) -> Result<Vec<u8>, CryptoError> {
    let raw = URL_SAFE
        .decode(token.trim().as_bytes())
        .or_else(|_| URL_SAFE_NO_PAD.decode(token.trim().as_bytes()))
        .map_err(|_| CryptoError)?;
    if raw.len() < 1 + 8 + 16 + 32 || raw[0] != 0x80 {
        return Err(CryptoError);
    }
    let (body, sig) = raw.split_at(raw.len() - 32);
    let signing = &key[..16];
    let encryption = &key[16..];
    let mut mac = HmacSha256::new_from_slice(signing).map_err(|_| CryptoError)?;
    mac.update(body);
    mac.verify_slice(sig).map_err(|_| CryptoError)?;

    let iv: [u8; 16] = body[9..25].try_into().map_err(|_| CryptoError)?;
    let mut ciphertext = body[25..].to_vec();
    let plain = Aes128CbcDec::new(encryption.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut ciphertext)
        .map_err(|_| CryptoError)?
        .to_vec();
    Ok(plain)
}

pub fn encrypt_secret(secret_key: &str, plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_fernet_key(secret_key);
    fernet_encrypt(&key, plaintext.as_bytes())
}

pub fn decrypt_secret(secret_key: &str, token: &str) -> Result<String, CryptoError> {
    let key = derive_fernet_key(secret_key);
    let bytes = fernet_decrypt(&key, token)?;
    String::from_utf8(bytes).map_err(|_| CryptoError)
}

/// Decrypt a stored secret. Values that are not Fernet tokens are returned as-is
/// so databases written by the Python build (plaintext fallback) still send mail.
pub fn decrypt_or_plain(secret_key: &str, value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    decrypt_secret(secret_key, value).unwrap_or_else(|_| value.to_string())
}

pub fn hash_api_key(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}

pub fn generate_api_key() -> (String, String, String) {
    let mut bytes = [0u8; 28];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let raw = format!("obs_{}", hex::encode(bytes));
    let hash = hash_api_key(&raw);
    let prefix = raw[..10].to_string();
    (raw, hash, prefix)
}

pub fn generate_heartbeat_token() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, 12)
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    bcrypt::verify(password, password_hash).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fernet_roundtrip_and_wrong_key() {
        let key = derive_fernet_key("test-secret");
        let token = fernet_encrypt(&key, b"smtp-password").unwrap();
        assert_eq!(fernet_decrypt(&key, &token).unwrap(), b"smtp-password");
        let other = derive_fernet_key("other-secret");
        assert!(fernet_decrypt(&other, &token).is_err());
    }

    #[test]
    fn official_fernet_vector() {
        // https://github.com/fernet/spec/blob/master/Spec.md
        let key_b64 = "cw_0x689RpI-jtRR7oE8h_eQsKImvJapLeSbXpwF4e4=";
        let key_bytes = URL_SAFE.decode(key_b64).unwrap();
        let key: [u8; 32] = key_bytes.as_slice().try_into().unwrap();
        let token = "gAAAAAAdwJ6wAAECAwQFBgcICQoLDA0ODy021cpGVWKZ_eEwCGM4BLLF_5CV9dOPmrhuVUPgJobwOz7JcbmrR64jVmpU4IwqDA==";
        let plain = fernet_decrypt(&key, token).unwrap();
        assert_eq!(plain, b"hello");
    }

    #[test]
    fn api_key_shape() {
        let (raw, hash, prefix) = generate_api_key();
        assert!(raw.starts_with("obs_"));
        assert_eq!(raw.len(), 60);
        assert_eq!(hash.len(), 64);
        assert_eq!(prefix, raw[..10]);
        assert_eq!(hash, hash_api_key(&raw));
        assert_eq!(generate_heartbeat_token().len(), 32);
    }

    #[test]
    fn password_hash_verifies() {
        let hash = hash_password("admin").unwrap();
        assert!(verify_password("admin", &hash));
        assert!(!verify_password("nope", &hash));
        assert!(!verify_password("admin", "not-a-hash"));
    }
}
