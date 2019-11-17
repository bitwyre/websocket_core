use super::{AuthField, AuthLocation};

impl<'a> AuthField<'a> {
    pub fn jwt(token: &'a str) -> Self {
        Self { key_or_token: token }
    }
}

impl<'a> From<AuthField<'a>> for AuthLocation<'a> {
    fn from(field: AuthField<'a>) -> Self {
        Self::WebSocketFrame(field)
    }
}
