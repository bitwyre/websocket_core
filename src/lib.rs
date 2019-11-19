pub extern crate actix;
pub extern crate actix_codec;
pub extern crate actix_rt;
pub extern crate actix_server;
pub extern crate actix_web;
pub extern crate actix_web_actors;
pub extern crate chrono;
pub extern crate crossbeam_channel;
pub extern crate crossbeam_utils;
pub extern crate env_logger;
pub extern crate futures;
pub extern crate futures_locks;
pub extern crate mimalloc;
pub extern crate openssl;
pub extern crate sentry;
pub extern crate url;
pub extern crate uuid;

mod broadcast_periodic;
mod broadcast_pubsub;
mod common_types;
mod env_helper;
mod reactive;
mod traits;

// Traits Re-export
pub use traits::{
    AuthMarker, PeriodicStringComposerRef, PeriodicWebsocketConfig, ReactiveStringHandlerRef, ReactiveWebsocketConfig,
    UpgradeWithAuthHandlerRef, WebsocketConfig,
};

pub use broadcast_periodic::{run_periodic_websocket_service, PeriodicWebsocketState};
pub use broadcast_pubsub::{
    run_pubsub_websocket_service, BroadcastMessage, PubsubWebsocketConfig, PubsubWebsocketState, SendBroadcastFunction,
    StaticStateArc,
};
pub use common_types::{CommonResponse, JsonSerializable, WebsocketServiceType};
pub use env_helper::{
    get_env_bool, get_env_int, get_env_string, get_executable_name, get_mandatory_env_bool, get_mandatory_env_int,
    get_mandatory_env_string,
};
pub use log::{debug, error, info, trace, warn};
pub use reactive::{run_reactive_websocket_service, ReactiveWebsocketState};
pub use sentry::internals::ClientInitGuard;

use std::env;

pub(crate) const ACTOR_MAILBOX_CAPACITY: usize = 1024;
pub const NOTFOUND_MESSAGE: &str = "You won't find anything here!";

pub(crate) fn exit_with_error(error_message: &str) -> ! {
    error!("{}", error_message);
    panic!("{}", error_message)
}

#[inline]
pub fn init_log(debug_mode: bool, sentry_dsn: Option<String>) -> Option<ClientInitGuard> {
    env::set_var("RUST_LOG", if debug_mode { "debug" } else { "info" });
    env_logger::builder()
        .default_format()
        .format_timestamp_nanos()
        .format_indent(Some(4))
        .init();
    match (debug_mode, sentry_dsn) {
        (false, Some(dsn)) => {
            let sentry_guard = sentry::init((
                dsn,
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    ..Default::default()
                },
            ));
            sentry::integrations::panic::register_panic_handler();
            Some(sentry_guard)
        }
        _ => None,
    }
}
