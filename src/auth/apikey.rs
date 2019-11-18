use super::{ActixResult, ErrorUnauthorized};
use crate::error;
use biscuit::{jwa::SignatureAlgorithm, jws::Secret};
use serde_json::Value;

pub(crate) struct Data {
    pub uri_path: String,
    pub payload: Value,
    pub nonce: u64,
}

impl Data {
    pub(crate) fn validate(self, secret: Vec<u8>, expected_signature: &[u8]) -> ActixResult<()> {
        let data = {
            let nonce = self.nonce.to_string();
            let compact_json_payload = &format!("{}", self.payload);
            self.uri_path + &sha256(&(nonce + &sha256(compact_json_payload)?))?
        };

        SignatureAlgorithm::HS512
            .verify(expected_signature, data.as_bytes(), &Secret::Bytes(secret))
            .map_err(ErrorUnauthorized)
    }
}

fn sha256(data: &str) -> ActixResult<String> {
    let checksum = openssl::sha::sha256(data.as_bytes());
    std::str::from_utf8(&checksum).map(str::to_string).map_err(|e| {
        error!("checksum not utf-8");
        ErrorUnauthorized(e)
    })
}
