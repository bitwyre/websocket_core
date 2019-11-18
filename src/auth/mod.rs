pub(super) use crate::actix_web::Result as ActixResult;
use crate::actix_web::{error::ErrorUnauthorized, http::HeaderMap};
use crate::common_types::Request;
use serde_json as json;

pub mod apikey;
pub mod jwt;
mod location;

pub use location::*;

pub enum AuthMode<'a> {
    JWT {
        auth_location: AuthLocation<'a>,
        signing_secret: &'a [u8],
        validate: jwt::ClaimCode,
    },
    APIKey {
        auth_location: AuthLocation<'a>,
        signing_secret: &'a [u8],
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

    pub(crate) fn validate(&self, request: Request) -> ActixResult<()> {
        match self {
            Self::None => Ok(()),
            Self::JWT {
                auth_location: template,
                validate: claim_code,
                signing_secret: secret,
            } => {
                let token = match (template, &request) {
                    (AuthLocation::Header(template), Request::HttpHeader(headers)) => {
                        extract_token_from_header(template, headers)?
                    }
                    (AuthLocation::WebSocketFrame(field), Request::WebsocketFrame(ws_request)) => {
                        extract_token_from_wsframe(field.key_or_token, ws_request)?
                    }
                    _ => unreachable!("check your `ws_upgrader` or `Actor::handler` implementation"),
                };
                claim_code.validate(secret, token)
            }
            Self::APIKey { .. } => unreachable!("TODO"),
        }
    }
}

fn extract_token_from_header<'a>(template: &AuthHeader, header: &'a HeaderMap) -> ActixResult<&'a str> {
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

fn extract_token_from_wsframe<'a>(field: &str, dataframe: &'a json::Value) -> ActixResult<&'a str> {
    match dataframe {
        json::Value::Object(obj) => obj
            .get(field)
            .and_then(|s| s.as_str())
            .ok_or_else(|| ErrorUnauthorized(format!("\"{}\" not found or it's not a `string`", field))),
        _ => Err(ErrorUnauthorized("request must be in type object")),
    }
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
