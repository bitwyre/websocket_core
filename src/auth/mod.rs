pub(super) use crate::actix_web::Error as HttpError;
use crate::actix_web::HttpRequest;
use actix_web::http::header::HeaderMap;

mod jwt;

#[derive(Clone)]
pub enum Auth {
    JWT {
        /** Header where the authentication token reside
        the format is always to be `... {token} ...`.
        Default is `Authorization: Bearer {token}` */
        auth_header: &'static str,
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
            auth_header: "Authorization: Bearer {token}",
            validate_code: Default::default(),
        }
    }

    pub(crate) fn validate(&self, request: &HttpRequest) -> Result<(), HttpError> {
        match self {
            Self::None => Ok(()),
            Self::JWT {
                auth_header: template,
                validate_code,
            } => {
                let token = extract_token(template, request.headers());
                jwt::validate(token, validate_code)
            }
        }
    }
}

fn extract_token<'a>(template: &'static str, header: &HeaderMap) -> &'a str {
    unimplemented!()
}
