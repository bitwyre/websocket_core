pub(super) use crate::actix_web::Result;
use crate::actix_web::{error::ErrorUnauthorized, HttpRequest};
use actix_web::http::header::HeaderMap;
use std::convert::TryFrom;

mod jwt;

#[derive(Clone)]
pub struct AuthHeader {
    field: &'static str,
    token_bound: (Option<&'static str>, Option<&'static str>),
}

impl AuthHeader {
    /// return None if value is invalid or can't be parsed
    pub fn new(field: &'static str, value: &'static str) -> Option<Self> {
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

#[derive(Clone)]
pub enum Auth {
    JWT {
        /** Header where the authentication token reside.
        The format value is always be `... {token} ...`.
        Default is `Authorization: Bearer {token}` */
        auth_header: AuthHeader,
        validate_code: jwt::ClaimCode,
    },
    None,
}

impl Default for Auth {
    fn default() -> Self {
        Self::None
    }
}

impl Auth {
    pub fn default_jwt() -> Self {
        Self::JWT {
            auth_header: AuthHeader::new("Authorization", "Bearer {token}").expect("has {token}"),
            validate_code: Default::default(),
        }
    }

    pub(crate) fn validate(&self, request: &HttpRequest) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::JWT {
                auth_header: template,
                validate_code,
            } => {
                let token = extract_token(template, request.headers())?;
                jwt::validate(token, validate_code)
            }
        }
    }
}

fn extract_token<'a>(template: &AuthHeader, header: &'a HeaderMap) -> Result<&'a str> {
    let header_value = header.get(template.field).ok_or_else(|| {
        let message = ["Missing field '", template.field, "'"].concat();
        ErrorUnauthorized(message)
    })?;

    let mut token = header_value.to_str().map_err(|e| ErrorUnauthorized(e.to_string()))?;
    if let Some(non_token) = template.token_bound.0 {
        token = token.trim_start_matches(non_token);
    }
    if let Some(non_token) = template.token_bound.1 {
        token = token.trim_end_matches(non_token);
    }
    Ok(token)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::error::Error;
    use std::string::ParseError;

    #[test]
    fn test_instantiate_auth_header() {
        assert!(AuthHeader::new("Authorization", "Bearer token").is_none());
        let authorization = |value| AuthHeader::new("Authorization", value).unwrap().token_bound;
        assert_eq!((Some("Bearer "), None), authorization("Bearer {token}"));
        assert_eq!((None, Some(" Key")), authorization("{token} Key"));
        assert_eq!((Some("Bearer "), Some(" Key")), authorization("Bearer {token} Key"));
    }

    #[test]
    fn test_extract_token() -> Result<(), Box<dyn Error>> {
        const TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

        let auth_header = AuthHeader::new("Authorization", "Bearer {token}").expect("has {token}");
        let mut request_header = HeaderMap::new();

        request_header.insert("API-Key".parse()?, "12345".parse()?);
        request_header.insert("Authorization".parse()?, ["Bearer ", TOKEN].concat().parse()?);

        assert_eq!(TOKEN, extract_token(&auth_header, &request_header)?);
        assert!(extract_token(&auth_header, &HeaderMap::new()).is_err());
        Ok(())
    }
}
