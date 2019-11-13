//! How to run this and also generate public_key.der
//! 1. visit https://jwt.io and select `Algorithm: RS256`
//! 2. copy the public key into public_key.pm
//! 3. `openssl rsa -pubin -in public_key.pem -outform DER -out public_key.der -RSAPublicKey_out`
//! 4. `cargo run --example jwt_periodic_broadcast`
//! 5. enter browser console (CTRL+SHFT+K)
//!    run `parseInt((new Date().getTime() + 1 * 60 * 1000)/1000)` and copy the result
//!    it mean the token will expire in 1 minute
//! 6. add `exp` field with the previous number in the **PAYLOAD** text field
//!    For example
//!    {
//!        "sub": "1234567890",
//!        "name": "John Doe",
//!        "admin": true,
//!        "iat": 1516239022,
//!        "exp": 1573596610
//!    }
//! 7. copy the token from jwt.io **Encoded** text field
//! 8. `websocat ws://127.0.0.1:8080/ws/love --header="Authorization: Bearer ${TOKEN}"`

#[global_allocator]
static GLOBAL: bitwyre_ws_core::mimalloc::MiMalloc = bitwyre_ws_core::mimalloc::MiMalloc;

use bitwyre_ws_core::{init_log, jwt, run_periodic_websocket_service};
use bitwyre_ws_core::{AuthMode, AuthHeader, PeriodicWebsocketConfig, PeriodicWebsocketState};
use once_cell::sync::Lazy;
use std::{io, sync::Arc, time::Duration};

fn main() -> io::Result<()> {
    init_log(true, None);
    static STATE: Lazy<PeriodicWebsocketState> = Lazy::new(|| {
        PeriodicWebsocketState::new(PeriodicWebsocketConfig {
            binding_url: "0.0.0.0:8080".into(),
            binding_path: "/ws/love".into(),
            max_clients: 16384,
            periodic_interval: Duration::from_millis(1000),
            rapid_request_limit: Duration::from_millis(1000),
            periodic_message_getter: Arc::new(&|| "love".into()),
            auth: AuthMode::JWT {
                auth_header: AuthHeader::default(),
                signing_secret: include_bytes!("../public_key.der"),
                algorithm: jwt::SignatureAlgorithm::RS256,
                validate: jwt::ClaimCode {
                    exp: true,
                    ..Default::default()
                },
            },
        })
    });
    run_periodic_websocket_service(Arc::new(&STATE))
}
