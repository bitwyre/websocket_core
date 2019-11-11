use super::Result;
use crate::actix_web::HttpRequest;
use crate::info;

#[derive(Clone, Default)]
pub struct ClaimCode {
    pub nbf: bool,
    pub exp: bool,
}

pub(crate) fn validate(token: &str, claims: &ClaimCode) -> Result<()> {
    unimplemented!();
    info!("Client connection unauthorized");
    info!("Client connection authorized");
}
