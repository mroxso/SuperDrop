//! Single-use UKEY2 initiator for Nearby's P256/SHA512 suite.
use crate::{
    proto::{securegcm as g, securemessage as s},
    MAX_FRAME,
};
use hkdf::Hkdf;
use p256::{ecdh::diffie_hellman, elliptic_curve::sec1::ToEncodedPoint, PublicKey, SecretKey};
use prost::Message;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256, Sha512};
use std::io::{Error, ErrorKind, Result};
use zeroize::Zeroizing;
fn invalid() -> Error {
    Error::new(ErrorKind::InvalidData, "invalid UKEY2 server handshake")
}
fn wrap(kind: i32, data: Vec<u8>) -> Vec<u8> {
    g::Ukey2Message {
        message_type: Some(kind),
        message_data: Some(data),
    }
    .encode_to_vec()
}
fn signed_coordinate(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if bytes[0] & 128 != 0 {
        out.push(0);
    }
    out.extend_from_slice(bytes);
    out
}
fn coordinate(bytes: &[u8]) -> Result<[u8; 32]> {
    let bytes = if bytes.len() == 33 && bytes[0] == 0 {
        &bytes[1..]
    } else {
        bytes
    };
    if bytes.is_empty() || bytes.len() > 32 {
        return Err(invalid());
    }
    let mut out = [0; 32];
    out[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(out)
}
pub struct ClientHandshake {
    secret: SecretKey,
    init: Vec<u8>,
    finish: Vec<u8>,
}
pub struct HandshakeResult {
    pub client_finish: Vec<u8>,
    /// Compare out of band before treating the peer as authenticated.
    pub authentication: Zeroizing<[u8; 32]>,
    pub next_secret: Zeroizing<[u8; 32]>,
}
impl Default for ClientHandshake {
    fn default() -> Self {
        Self::new()
    }
}
impl ClientHandshake {
    pub fn new() -> Self {
        let secret = SecretKey::random(&mut OsRng);
        let point = secret.public_key().to_encoded_point(false);
        let public = s::GenericPublicKey {
            r#type: 1,
            ec_p256_public_key: Some(s::EcP256PublicKey {
                x: signed_coordinate(point.x().unwrap()),
                y: signed_coordinate(point.y().unwrap()),
            }),
            ..Default::default()
        }
        .encode_to_vec();
        let finish = wrap(
            4,
            g::Ukey2ClientFinished {
                public_key: Some(public),
            }
            .encode_to_vec(),
        );
        let mut random = vec![0; 32];
        OsRng.fill_bytes(&mut random);
        let init = wrap(
            2,
            g::Ukey2ClientInit {
                version: Some(1),
                random: Some(random),
                cipher_commitments: vec![g::ukey2_client_init::CipherCommitment {
                    handshake_cipher: Some(100),
                    commitment: Some(Sha512::digest(&finish).to_vec()),
                }],
                next_protocol: Some("AES_256_CBC-HMAC_SHA256".into()),
            }
            .encode_to_vec(),
        );
        Self {
            secret,
            init,
            finish,
        }
    }
    pub fn client_init(&self) -> &[u8] {
        &self.init
    }
    /// Consumes the state, so a server reply cannot cause key reuse.
    pub fn complete(self, server_wire: &[u8]) -> Result<HandshakeResult> {
        if server_wire.len() > MAX_FRAME {
            return Err(invalid());
        }
        let outer = g::Ukey2Message::decode(server_wire).map_err(|_| invalid())?;
        if outer.message_type != Some(3) {
            return Err(invalid());
        }
        let server = g::Ukey2ServerInit::decode(outer.message_data.ok_or_else(invalid)?.as_slice())
            .map_err(|_| invalid())?;
        if server.version != Some(1)
            || server.handshake_cipher != Some(100)
            || server.random.as_ref().map(Vec::len) != Some(32)
        {
            return Err(invalid());
        }
        let key = s::GenericPublicKey::decode(server.public_key.ok_or_else(invalid)?.as_slice())
            .map_err(|_| invalid())?;
        if key.r#type != 1 {
            return Err(invalid());
        }
        let point = key.ec_p256_public_key.ok_or_else(invalid)?;
        let mut sec1 = vec![4];
        sec1.extend_from_slice(&coordinate(&point.x)?);
        sec1.extend_from_slice(&coordinate(&point.y)?);
        let public = PublicKey::from_sec1_bytes(&sec1).map_err(|_| invalid())?;
        let shared = diffie_hellman(self.secret.to_nonzero_scalar(), public.as_affine());
        let dhs = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(shared.raw_secret_bytes())));
        let mut transcript = self.init;
        transcript.extend_from_slice(server_wire);
        let mut authentication = Zeroizing::new([0; 32]);
        let mut next_secret = Zeroizing::new([0; 32]);
        Hkdf::<Sha256>::new(Some(b"UKEY2 v1 auth\0"), dhs.as_ref())
            .expand(&transcript, authentication.as_mut())
            .map_err(|_| invalid())?;
        Hkdf::<Sha256>::new(Some(b"UKEY2 v1 next"), dhs.as_ref())
            .expand(&transcript, next_secret.as_mut())
            .map_err(|_| invalid())?;
        Ok(HandshakeResult {
            client_finish: self.finish,
            authentication,
            next_secret,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_type_and_invalid_curve() {
        assert!(ClientHandshake::new().complete(&wrap(1, vec![])).is_err());
        let public = s::GenericPublicKey {
            r#type: 1,
            ec_p256_public_key: Some(s::EcP256PublicKey {
                x: vec![0; 32],
                y: vec![0; 32],
            }),
            ..Default::default()
        }
        .encode_to_vec();
        let server = wrap(
            3,
            g::Ukey2ServerInit {
                version: Some(1),
                random: Some(vec![0; 32]),
                handshake_cipher: Some(100),
                public_key: Some(public),
            }
            .encode_to_vec(),
        );
        assert!(ClientHandshake::new().complete(&server).is_err());
    }
}
