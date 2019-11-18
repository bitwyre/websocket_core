use super::{AuthField, AuthLocation};

impl<'a> AuthField<'a> {
    pub fn jwt(token: &'a str) -> Self {
        Self {
            key_or_token: token,
            sign: None,
            payload: None,
        }
    }

    pub fn apikey(key: &'a str, signature: &'a str, payload_key: &'a str) -> Self {
        Self {
            key_or_token: key,
            sign: Some(signature),
            payload: Some(payload_key),
        }
    }
}

impl<'a> From<AuthField<'a>> for AuthLocation<'a> {
    fn from(field: AuthField<'a>) -> Self {
        Self::WebSocketFrame(field)
    }
}
