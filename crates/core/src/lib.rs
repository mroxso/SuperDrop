//! Independent Quick Share QR activation primitives. No payload transport yet.
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes128Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hkdf::Hkdf;
use p256::{elliptic_curve::sec1::ToEncodedPoint, SecretKey};
use rand_core::OsRng;
use sha2::Sha256;
use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};

pub const SERVICE: &str = "_FC9F5ED42C8A._tcp.local.";
pub const MAX_FRAME: usize = 1024 * 1024;

/// Retains ephemeral key material only in memory. Debug intentionally omitted.
pub struct QrSession {
    _secret: SecretKey,
    encoded: Vec<u8>,
    token: [u8; 16],
    name_key: [u8; 16],
    created: Instant,
}
impl Default for QrSession {
    fn default() -> Self {
        Self::new()
    }
}
impl QrSession {
    pub fn new() -> Self {
        let secret = SecretKey::random(&mut OsRng);
        let mut encoded = vec![0, 0];
        encoded.extend_from_slice(secret.public_key().to_encoded_point(true).as_bytes());
        let hk = Hkdf::<Sha256>::new(None, &encoded);
        let mut token = [0; 16];
        let mut name_key = [0; 16];
        hk.expand(b"advertisingContext", &mut token)
            .expect("fixed HKDF length");
        hk.expand(b"encryptionKey", &mut name_key)
            .expect("fixed HKDF length");
        Self {
            _secret: secret,
            encoded,
            token,
            name_key,
            created: Instant::now(),
        }
    }
    pub fn expired(&self) -> bool {
        self.created.elapsed() >= Duration::from_secs(120)
    }
    pub fn url(&self) -> Option<String> {
        (!self.expired()).then(|| {
            format!(
                "https://quickshare.google/qrcode#key={}",
                URL_SAFE_NO_PAD.encode(&self.encoded)
            )
        })
    }
    /// Returns a name only for advertisements cryptographically bound to this QR.
    pub fn match_endpoint(&self, bytes: &[u8]) -> Option<String> {
        if self.expired() || bytes.len() < 18 || bytes.len() > 1024 {
            return None;
        }
        let hidden = bytes[0] & 0x10 != 0;
        let mut pos = 17;
        let name = if !hidden {
            let len = *bytes.get(pos)? as usize;
            pos += 1;
            let name = std::str::from_utf8(bytes.get(pos..pos + len)?)
                .ok()?
                .to_owned();
            pos += len;
            Some(name)
        } else {
            None
        };
        while pos < bytes.len() {
            let kind = *bytes.get(pos)?;
            let len = *bytes.get(pos + 1)? as usize;
            pos += 2;
            let value = bytes.get(pos..pos + len)?;
            pos += len;
            if kind != 1 {
                continue;
            }
            if !hidden && value == self.token {
                return name;
            }
            if hidden && value.len() >= 28 {
                let cipher = Aes128Gcm::new_from_slice(&self.name_key).ok()?;
                let plain = cipher
                    .decrypt(
                        &Nonce::from(<[u8; 12]>::try_from(&value[..12]).ok()?),
                        Payload {
                            msg: &value[12..],
                            aad: &self.token,
                        },
                    )
                    .ok()?;
                return String::from_utf8(plain).ok();
            }
        }
        None
    }
}

pub fn read_frame(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix)?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame exceeds bounds",
        ));
    }
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}
pub fn write_frame(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame exceeds bounds",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_bounds() {
        assert!(read_frame(&mut &u32::MAX.to_be_bytes()[..]).is_err());
        assert!(read_frame(&mut &[0, 0, 0, 2, 1][..]).is_err());
        let mut wire = Vec::new();
        write_frame(&mut wire, b"hello").unwrap();
        assert_eq!(read_frame(&mut wire.as_slice()).unwrap(), b"hello");
    }
    #[test]
    fn binding_and_malformed_metadata() {
        let session = QrSession::new();
        let mut endpoint = vec![0; 17];
        endpoint.extend_from_slice(&[3, b'p', b'h', b'n', 1, 16]);
        endpoint.extend_from_slice(&session.token);
        assert_eq!(session.match_endpoint(&endpoint).as_deref(), Some("phn"));
        assert!(QrSession::new().match_endpoint(&endpoint).is_none());
        for length in 0..endpoint.len() {
            assert!(session.match_endpoint(&endpoint[..length]).is_none());
        }
        assert_eq!(session.encoded.len(), 35);
    }
}
