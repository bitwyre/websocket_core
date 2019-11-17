mod header;
mod field;

pub enum AuthLocation<'a> {
    Header(AuthHeader<'a>),
    WebSocketFrame(AuthField<'a>),
}

pub struct AuthHeader<'a> {
    pub(crate) field: &'a str,
    pub(crate) token_bound: (Option<&'a str>, Option<&'a str>),
}

pub struct AuthField<'a> {
    pub(crate) key_or_token: &'a str,
}
