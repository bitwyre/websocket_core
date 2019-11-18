mod header;
mod field;

#[derive(Clone)]
pub enum AuthLocation<'a> {
    Header(AuthHeader<'a>),
    WebSocketFrame(AuthField<'a>),
}

#[derive(Clone)]
pub struct AuthHeader<'a> {
    pub(crate) field: &'a str,
    pub(crate) token_bound: (Option<&'a str>, Option<&'a str>),
}

#[derive(Clone)]
pub struct AuthField<'a> {
    pub(crate) key_or_token: &'a str,
    pub(crate) sign: Option<&'a str>,
}
