use super::{AuthHeader, AuthLocation};

impl<'a> AuthHeader<'a> {
    /// return None if value is invalid or can't be parsed
    pub fn new(field: &'a str, value: &'a str) -> Option<Self> {
        let mut not_token = value.trim().split("{token}");
        let token_bound = (
            not_token.next().filter(|s| !s.is_empty()),
            match not_token.next() {
                None => return None,
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s),
            },
        );
        Some(Self { field, token_bound })
    }
}

impl Default for AuthHeader<'_> {
    fn default() -> Self {
        AuthHeader::new("Authorization", "Bearer {token}").expect("has {token}")
    }
}

impl<'a> From<AuthHeader<'a>> for AuthLocation<'a> {
    fn from(header: AuthHeader<'a>) -> Self {
        Self::Header(header)
    }
}
