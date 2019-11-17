pub(super) use crate::actix_web::Result as ActixResult;
use crate::actix_web::{error::ErrorUnauthorized, HttpRequest};
use actix_web::http::header::HeaderMap;

pub mod jwt;
mod location;

pub use location::*;

// #[derive(Clone)]
pub enum AuthMode<'a> {
    JWT {
        auth_location: AuthLocation<'a>,
        signing_secret: &'a [u8],
        validate: jwt::ClaimCode,
    },
    None,
}

impl Default for AuthMode<'_> {
    fn default() -> Self {
        Self::None
    }
}

impl AuthMode<'_> {
    pub fn default_jwt_from(signing_secret: &'static [u8]) -> Self {
        let auth_header = AuthHeader::new("Authorization", "Bearer {token}").expect("has {token}");
        Self::JWT {
            auth_location: AuthLocation::from(auth_header),
            validate: jwt::ClaimCode::disable_all(),
            signing_secret,
        }
    }

    pub(crate) fn validate(&self, request: &HttpRequest) -> ActixResult<()> {
        match self {
            Self::None => Ok(()),
            Self::JWT {
                auth_location: template,
                validate: claim_code,
                signing_secret: secret,
            } => {
                let token = match template {
                    AuthLocation::Header(template) => extract_token(template, request.headers())?,
                    AuthLocation::WebSocketFrame(field) => field.key_or_token,
                };
                claim_code.validate(secret, token)
            }
        }
    }
}

fn extract_token<'a>(template: &AuthHeader, header: &'a HeaderMap) -> ActixResult<&'a str> {
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
        const TOKEN: &str = include_str!("../../test/fixture/jwt_token.key");

        let auth_header = AuthHeader::new("Authorization", "Bearer {token}").expect("has {token}");
        let mut request_header = HeaderMap::new();

        request_header.insert("API-Key".parse()?, "12345".parse()?);
        request_header.insert("Authorization".parse()?, ["Bearer ", TOKEN].concat().parse()?);

        assert_eq!(TOKEN, extract_token(&auth_header, &request_header)?);
        assert!(extract_token(&auth_header, &HeaderMap::new()).is_err());
        Ok(())
    }
}
