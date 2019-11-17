use crate::error;
use biscuit::{jwa::SignatureAlgorithm, jws::Secret};
use serde_json::Value;

fn last_nonce_requested(api_key: &str) -> Option<u64> {
    unimplemented!()
}

pub struct Data {
    uri_path: String,
    payload: Value,
    nonce: u64,
}

impl Data {
    pub(crate) fn validate(self, secret: Vec<u8>, expected_signature: &[u8], api_key: &str) {
        let data = {
            let nonce = self.nonce.to_string();
            let compact_json_payload = &format!("{}", self.payload);
            self.uri_path + &sha256(&(nonce + &sha256(compact_json_payload)))
        };

        SignatureAlgorithm::HS512
            .verify(expected_signature, data.as_bytes(), &Secret::Bytes(secret))
            .expect("valid");

        fn sha256(data: &str) -> String {
            let checksum = openssl::sha::sha256(data.as_bytes());
            std::str::from_utf8(&checksum).map(str::to_string).unwrap_or_default()
        }
    }
}
