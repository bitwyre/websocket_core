use crate::actix_web::HttpRequest;
use std::sync::Arc;
use std::time::Duration;

pub type PeriodicStringComposerRef = Arc<&'static (dyn Fn() -> String + Sync + Send)>;
pub type ReactiveStringHandlerRef = Arc<&'static (dyn Fn(String) -> Option<String> + Sync + Send)>;
pub type AuthMarker = Option<String>;
pub type UpgradeWithAuthHandlerRef = Arc<&'static (dyn Fn(&HttpRequest) -> AuthMarker + Sync + Send)>;

pub trait WebsocketConfig {
    fn get_binding_url(&self) -> String;
    fn get_binding_path(&self) -> String;
    fn get_flag_check_auth(&self) -> bool {
        false
    }
    fn get_max_clients(&self) -> usize {
        16_384
    }
    fn get_slow_client_timeout(&self) -> Duration {
        Duration::from_millis(250)
    }
    fn get_rapid_request_limit(&self) -> Option<Duration> {
        None
    }
    fn get_upgrade_with_auth_handler(&self) -> Option<UpgradeWithAuthHandlerRef> {
        if self.get_flag_check_auth() {
            panic!("To use authentication, please override \"get_upgrade_with_auth_hander\"")
        }
        None
    }
}

pub trait PeriodicWebsocketConfig<T = Self>
where
    T: WebsocketConfig,
{
    fn get_periodic_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn get_periodic_message_composer(&self) -> PeriodicStringComposerRef;
}

pub trait ReactiveWebsocketConfig<T = Self>
where
    T: WebsocketConfig,
{
    fn get_message_handler(&self) -> ReactiveStringHandlerRef;
}

#[cfg(test)]
mod unit_test {
    use super::*;
    use std::panic::catch_unwind;

    #[test]
    fn test_ws_config_panic_when_auth_enabled_but_no_upgrade_with_auth_handler_defined() {
        struct SomeConfigWithAuth;
        impl WebsocketConfig for SomeConfigWithAuth {
            fn get_binding_url(&self) -> String {
                "0.0.0.0:4000".to_owned()
            }
            fn get_binding_path(&self) -> String {
                "/ws".to_owned()
            }
            fn get_flag_check_auth(&self) -> bool {
                true
            }
        }
        let some_config_with_auth = SomeConfigWithAuth;
        let result = catch_unwind(|| {
            let _ = some_config_with_auth.get_upgrade_with_auth_handler();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_reactive_ws_config_message_handler_executable() {
        fn echo_message_handler(message: String) -> Option<String> {
            Some(message)
        }
        struct SomeReactiveConfig;
        impl WebsocketConfig for SomeReactiveConfig {
            fn get_binding_url(&self) -> String {
                "0.0.0.0:4000".to_owned()
            }
            fn get_binding_path(&self) -> String {
                "/ws".to_owned()
            }
        }
        impl ReactiveWebsocketConfig for SomeReactiveConfig {
            fn get_message_handler(&self) -> ReactiveStringHandlerRef {
                Arc::new(&echo_message_handler)
            }
        }
        let message_handler = SomeReactiveConfig.get_message_handler();
        assert_eq!(message_handler("SomeMessage".to_owned()).unwrap(), "SomeMessage");
    }
}
