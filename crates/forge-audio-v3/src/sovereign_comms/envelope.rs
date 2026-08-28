use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use k256::{ecdh::diffie_hellman, PublicKey, SecretKey};
use rand_core::{OsRng, RngCore};
use sha2::Sha256;

const VERSION: u8 = 1;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const OVERHEAD: usize = 1 + SALT_LEN + NONCE_LEN + 16; // version + salt + nonce + AEAD tag

/// Encrypted payload. Nostr carries this as opaque bytes - relays see nothing.
///
/// Key agreement: ECDH(secp256k1) -> HKDF-SHA256 -> ChaCha20-Poly1305.
/// Wire format: version(1) + salt(32) + nonce(12) + ciphertext+tag(n+16).
#[derive(Debug, Clone)]
pub struct OpaqueEnvelope {
    version: u8,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
    ciphertext: Vec<u8>,
}

impl serde::Serialize for OpaqueEnvelope {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> serde::Deserialize<'de> for OpaqueEnvelope {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(d)?;
        Self::from_bytes(&bytes).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[derive(Debug)]
pub enum EnvelopeError {
    Ecdh(String),
    Kdf,
    Encrypt,
    Decrypt,
    Decode(String),
    Version(u8),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::Ecdh(s) => write!(f, "ecdh: {}", s),
            EnvelopeError::Kdf => write!(f, "kdf: expand failed"),
            EnvelopeError::Encrypt => write!(f, "encrypt: aead failed"),
            EnvelopeError::Decrypt => write!(f, "decrypt: authentication failed"),
            EnvelopeError::Decode(s) => write!(f, "decode: {}", s),
            EnvelopeError::Version(v) => write!(f, "version mismatch: got {}", v),
        }
    }
}

impl OpaqueEnvelope {
    pub fn seal(
        sender_sk: &SecretKey,
        recipient_pk: &PublicKey,
        plaintext: &[u8],
    ) -> Result<Self, EnvelopeError> {
        let shared = diffie_hellman(sender_sk.to_nonzero_scalar(), recipient_pk.as_affine());

        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);

        let key_bytes = derive_key(shared.raw_secret_bytes().as_slice(), &salt)?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
            .map_err(|_| EnvelopeError::Encrypt)?;

        Ok(Self { version: VERSION, salt, nonce: nonce_bytes, ciphertext })
    }

    pub fn open(
        &self,
        recipient_sk: &SecretKey,
        sender_pk: &PublicKey,
    ) -> Result<Vec<u8>, EnvelopeError> {
        if self.version != VERSION {
            return Err(EnvelopeError::Version(self.version));
        }
        let shared = diffie_hellman(recipient_sk.to_nonzero_scalar(), sender_pk.as_affine());
        let key_bytes = derive_key(shared.raw_secret_bytes().as_slice(), &self.salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        cipher
            .decrypt(Nonce::from_slice(&self.nonce), self.ciphertext.as_slice())
            .map_err(|_| EnvelopeError::Decrypt)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(OVERHEAD + self.ciphertext.len());
        out.push(self.version);
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EnvelopeError> {
        if bytes.len() < OVERHEAD {
            return Err(EnvelopeError::Decode("too short".into()));
        }
        let version = bytes[0];
        let salt: [u8; SALT_LEN] = bytes[1..1 + SALT_LEN]
            .try_into()
            .map_err(|_| EnvelopeError::Decode("salt slice".into()))?;
        let nonce: [u8; NONCE_LEN] = bytes[1 + SALT_LEN..1 + SALT_LEN + NONCE_LEN]
            .try_into()
            .map_err(|_| EnvelopeError::Decode("nonce slice".into()))?;
        let ciphertext = bytes[1 + SALT_LEN + NONCE_LEN..].to_vec();
        Ok(Self { version, salt, nonce, ciphertext })
    }
}

fn derive_key(shared: &[u8], salt: &[u8]) -> Result<[u8; 32], EnvelopeError> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), shared);
    let mut key = [0u8; 32];
    hkdf.expand(b"forge-envelope-v1", &mut key).map_err(|_| EnvelopeError::Kdf)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::SecretKey;

    fn keypair() -> (SecretKey, PublicKey) {
        let sk = SecretKey::random(&mut OsRng);
        let pk = sk.public_key();
        (sk, pk)
    }

    #[test]
    fn seal_open_roundtrip() {
        let (alice_sk, alice_pk) = keypair();
        let (bob_sk, bob_pk) = keypair();
        let msg = b"sovereign pirate radio covert channel";
        let env = OpaqueEnvelope::seal(&alice_sk, &bob_pk, msg).unwrap();
        let plain = env.open(&bob_sk, &alice_pk).unwrap();
        assert_eq!(plain, msg);
    }

    #[test]
    fn wrong_key_fails() {
        let (alice_sk, _) = keypair();
        let (_, bob_pk) = keypair();
        let (eve_sk, eve_pk) = keypair();
        let env = OpaqueEnvelope::seal(&alice_sk, &bob_pk, b"secret").unwrap();
        assert!(env.open(&eve_sk, &eve_pk).is_err());
    }

    #[test]
    fn bytes_roundtrip() {
        let (alice_sk, alice_pk) = keypair();
        let (bob_sk, bob_pk) = keypair();
        let msg = b"bytes roundtrip test";
        let env = OpaqueEnvelope::seal(&alice_sk, &bob_pk, msg).unwrap();
        let env2 = OpaqueEnvelope::from_bytes(&env.to_bytes()).unwrap();
        let plain = env2.open(&bob_sk, &alice_pk).unwrap();
        assert_eq!(plain, msg);
    }

    #[test]
    fn short_bytes_fails() {
        assert!(OpaqueEnvelope::from_bytes(&[0u8; 10]).is_err());
    }
}
