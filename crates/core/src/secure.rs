//! Authenticated Nearby secure envelopes. Instances belong to one connection.
use crate::{
    proto::{securegcm as g, securemessage as s},
    MAX_FRAME,
};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use prost::Message;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::io::{Error, ErrorKind, Result};
use zeroize::Zeroize;

fn invalid() -> Error {
    Error::new(ErrorKind::InvalidData, "invalid authenticated envelope")
}
fn derive(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let mut out = [0; 32];
    Hkdf::<Sha256>::new(Some(salt), secret)
        .expand(info, &mut out)
        .expect("fixed HKDF size");
    out
}
struct Keys {
    enc: [u8; 32],
    mac: [u8; 32],
}
impl Drop for Keys {
    fn drop(&mut self) {
        self.enc.zeroize();
        self.mac.zeroize();
    }
}
impl Keys {
    fn new(secret: &[u8], role: &[u8]) -> Self {
        let mut d2d = derive(secret, &Sha256::digest(b"D2D"), role);
        let salt = Sha256::digest(b"SecureMessage");
        let keys = Self {
            enc: derive(&d2d, &salt, b"ENC:2"),
            mac: derive(&d2d, &salt, b"SIG:1"),
        };
        d2d.zeroize();
        keys
    }
}
pub struct SecureChannel {
    send: Keys,
    receive: Keys,
    sent: i32,
    received: i32,
    failed: bool,
}
impl SecureChannel {
    pub fn new(next_secret: &[u8; 32], client: bool) -> Self {
        let (send, receive) = if client {
            (b"client".as_slice(), b"server".as_slice())
        } else {
            (b"server".as_slice(), b"client".as_slice())
        };
        Self {
            send: Keys::new(next_secret, send),
            receive: Keys::new(next_secret, receive),
            sent: 0,
            received: 0,
            failed: false,
        }
    }
    pub fn seal(&mut self, message: &[u8]) -> Result<Vec<u8>> {
        if self.failed || message.len() > MAX_FRAME - 1024 {
            return Err(invalid());
        }
        let sequence = self.sent.checked_add(1).ok_or_else(invalid)?;
        let plain = g::DeviceToDeviceMessage {
            message: Some(message.to_vec()),
            sequence_number: Some(sequence),
        }
        .encode_to_vec();
        let mut iv = [0; 16];
        OsRng.fill_bytes(&mut iv);
        let body = cbc::Encryptor::<aes::Aes256>::new((&self.send.enc).into(), (&iv).into())
            .encrypt_padded_vec_mut::<Pkcs7>(&plain);
        let metadata = g::GcmMetadata {
            r#type: 13,
            version: Some(1),
        }
        .encode_to_vec();
        let header = s::Header {
            signature_scheme: 1,
            encryption_scheme: 2,
            iv: Some(iv.to_vec()),
            public_metadata: Some(metadata),
            ..Default::default()
        };
        let header_and_body = s::HeaderAndBody { header, body }.encode_to_vec();
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.send.mac).expect("HMAC size");
        mac.update(&header_and_body);
        self.sent = sequence;
        Ok(s::SecureMessage {
            header_and_body,
            signature: mac.finalize().into_bytes().to_vec(),
        }
        .encode_to_vec())
    }
    pub fn open(&mut self, wire: &[u8]) -> Result<Vec<u8>> {
        if self.failed {
            return Err(invalid());
        }
        let result = self.open_inner(wire);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn open_inner(&mut self, wire: &[u8]) -> Result<Vec<u8>> {
        if wire.len() > MAX_FRAME {
            return Err(invalid());
        }
        let envelope = s::SecureMessage::decode(wire).map_err(|_| invalid())?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.receive.mac).expect("HMAC size");
        mac.update(&envelope.header_and_body);
        mac.verify_slice(&envelope.signature)
            .map_err(|_| invalid())?;
        let hb =
            s::HeaderAndBody::decode(envelope.header_and_body.as_slice()).map_err(|_| invalid())?;
        let h = hb.header;
        if h.signature_scheme != 1
            || h.encryption_scheme != 2
            || h.associated_data_length.unwrap_or(0) != 0
        {
            return Err(invalid());
        }
        let metadata = g::GcmMetadata::decode(h.public_metadata.ok_or_else(invalid)?.as_slice())
            .map_err(|_| invalid())?;
        if metadata.r#type != 13 || metadata.version != Some(1) {
            return Err(invalid());
        }
        let iv: [u8; 16] =
            h.iv.ok_or_else(invalid)?
                .try_into()
                .map_err(|_| invalid())?;
        let plain = cbc::Decryptor::<aes::Aes256>::new((&self.receive.enc).into(), (&iv).into())
            .decrypt_padded_vec_mut::<Pkcs7>(&hb.body)
            .map_err(|_| invalid())?;
        let msg = g::DeviceToDeviceMessage::decode(plain.as_slice()).map_err(|_| invalid())?;
        let expected = self.received.checked_add(1).ok_or_else(invalid)?;
        if msg.sequence_number != Some(expected) {
            return Err(invalid());
        }
        let payload = msg.message.ok_or_else(invalid)?;
        self.received = expected;
        Ok(payload)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directions_replay_and_tamper() {
        let mut client = SecureChannel::new(&[42; 32], true);
        let mut server = SecureChannel::new(&[42; 32], false);
        let wire = client.seal(b"hello").unwrap();
        assert_eq!(server.open(&wire).unwrap(), b"hello");
        assert_eq!(
            client.open(&server.seal(b"reply").unwrap()).unwrap(),
            b"reply"
        );
        assert!(server.open(&wire).is_err());
        assert!(server.open(&client.seal(b"next").unwrap()).is_err());
        let mut peer = SecureChannel::new(&[42; 32], false);
        let mut bad = wire;
        *bad.last_mut().unwrap() ^= 1;
        assert!(peer.open(&bad).is_err());
    }
}
