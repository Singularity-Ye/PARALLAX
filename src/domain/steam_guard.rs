//! Steam's login-code encoding, with cryptography delegated to ring.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
pub struct GuardSecret(Vec<u8>);
impl GuardSecret {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() > 128 {
            return Err("shared_secret格式无效".into());
        }
        let bytes = STANDARD
            .decode(raw)
            .map_err(|_| "shared_secret必须为标准Base64")?;
        if bytes.len() != 20 {
            return Err("Steam shared_secret解码后必须为20字节".into());
        }
        Ok(Self(bytes))
    }
    pub(crate) fn encoded(&self) -> String {
        STANDARD.encode(&self.0)
    }
    pub fn code(&self, at: u64) -> GuardCode {
        let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, &self.0);
        let digest = ring::hmac::sign(&key, &(at / 30).to_be_bytes());
        let digest = digest.as_ref();
        let offset = (digest[19] & 15) as usize;
        let mut value =
            u32::from_be_bytes(digest[offset..offset + 4].try_into().unwrap()) & 0x7fff_ffff;
        let alphabet = b"23456789BCDFGHJKMNPQRTVWXY";
        let code = (0..5)
            .map(|_| {
                let c = alphabet[(value % 26) as usize] as char;
                value /= 26;
                c
            })
            .collect();
        GuardCode {
            code,
            generated_at: at,
            valid_until: at - at % 30 + 30,
            period_secs: 30,
            time_basis: "local_system_clock",
            scope: "login_code_only",
            automatic_login_available: false,
        }
    }
}
impl Drop for GuardSecret {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}
#[derive(Serialize)]
pub struct GuardCode {
    pub code: String,
    pub generated_at: u64,
    pub valid_until: u64,
    pub period_secs: u32,
    pub time_basis: &'static str,
    pub scope: &'static str,
    pub automatic_login_available: bool,
}
#[derive(Serialize)]
pub struct GuardStatus {
    pub steam_id: String,
    pub configured: bool,
    pub scope: &'static str,
    pub automatic_login_available: bool,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_guard_matches_legacy_protocol_vectors_and_thirty_second_boundaries() {
        let secret = GuardSecret::parse("MTIzNDU2Nzg5MDEyMzQ1Njc4OTA=").unwrap();
        for (at, expected) in [
            (0, "GG5F5"),
            (29, "GG5F5"),
            (30, "PV9M4"),
            (59, "PV9M4"),
            (60, "B26KJ"),
            (1111111109, "PY4YB"),
        ] {
            let c = secret.code(at);
            assert_eq!(c.code, expected);
            assert!(c.valid_until > at && c.valid_until - at <= 30);
            assert!(!c.automatic_login_available);
        }
        for invalid in ["", "not base64!", "YWJj", " MTIzNDU2Nzg5MDEyMzQ1Njc4OTA="] {
            assert!(GuardSecret::parse(invalid).is_err());
        }
    }
}
