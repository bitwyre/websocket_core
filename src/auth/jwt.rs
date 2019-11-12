pub use biscuit::jwa::SignatureAlgorithm;

use super::{ActixResult, ErrorUnauthorized};
use crate::info;
use biscuit::{jws::Secret, Empty, Validation, ValidationOptions, JWT};

#[derive(Clone, Default)]
pub struct ClaimCode {
    pub nbf: bool,
    pub exp: bool,
}

impl ClaimCode {
    pub fn disable_all() -> Self {
        Self::default()
    }

    pub(crate) fn validate(&self, secret: &[u8], token: &str, algo: SignatureAlgorithm) -> ActixResult<()> {
        let token = JWT::<Empty, Empty>::new_encoded(token);

        let token = token
            .into_decoded(&Secret::Bytes(secret.to_vec()), algo)
            .map_err(ErrorUnauthorized)?;
        let claims = &token.payload().map_err(ErrorUnauthorized)?.registered;

        let is_error = if claims.not_before.is_none() && self.nbf {
            info!("Client connection unauthorized because `nbf` claims code not found");
            true
        } else if claims.expiry.is_none() && self.exp {
            info!("Client connection unauthorized because `exp` claims code not found");
            true
        } else {
            false
        };
        if is_error {
            return Err(ErrorUnauthorized("wrong token"));
        }

        let with_options = ValidationOptions {
            not_before: self.nbf.into_validation(),
            expiry: self.exp.into_validation(),
            ..Default::default()
        };
        claims.validate(with_options).map_err(ErrorUnauthorized)?;
        if let Some(timestamp) = claims.not_before {
            info!("Client connection authorized not before {}", timestamp.to_rfc3339());
        }
        if let Some(timestamp) = claims.expiry {
            info!("Client connection authorized expire at {}", timestamp.to_rfc3339());
        }
        Ok(())
    }
}

trait IntoValidation<T> {
    fn into_validation(self) -> Validation<T>;
}
impl IntoValidation<()> for bool {
    fn into_validation(self) -> Validation<()> {
        if self {
            Validation::Validate(())
        } else {
            Validation::Ignored
        }
    }
}
