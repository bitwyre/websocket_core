use crate::actix::Actor as ActixActor;
use crate::actix::ActorContext;
use crate::actix::Running;
use crate::actix::StreamHandler;
use crate::actix_web::middleware;
use crate::actix_web::web;
use crate::actix_web::web::Data as ActixData;
use crate::actix_web::web::Payload;
use crate::actix_web::App as ActixApp;
use crate::actix_web::Error as HttpError;
use crate::actix_web::HttpRequest;
use crate::actix_web::HttpResponse;
use crate::actix_web::HttpServer as ActixHttpServer;
use crate::actix_web_actors::ws::start as ws_start;
use crate::actix_web_actors::ws::Message as WsMessage;
use crate::actix_web_actors::ws::ProtocolError as WsProtocolError;
use crate::actix_web_actors::ws::WebsocketContext;
use crate::auth::AuthMode;
use crate::common_types::CommonResponse;
use crate::debug;
use crate::futures::future::ok;
use crate::futures::prelude::*;
use crate::info;
use crate::ACTOR_MAILBOX_CAPACITY;
use crate::NOTFOUND_MESSAGE;
use std::collections::HashMap;
use std::io::Result as IOResult;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

pub struct ReactiveWebsocketConfig {
    pub binding_url: String,
    pub binding_path: String,
    pub max_clients: usize,
    pub rapid_request_limit: Option<Duration>,
    pub message_handler: Arc<&'static (dyn Fn(String) -> Option<String> + Sync + Send)>,
    pub auth: AuthMode<'static>,
}

pub struct ReactiveWebsocketState {
    pub active_clients: AtomicUsize,
    pub rejection_counter: AtomicUsize,
    pub config: ReactiveWebsocketConfig,
}

pub(crate) struct ReactiveActor {
    rapid_request_rejection_enabled: bool,
    last_request_stopwatch: Instant,
    rapid_request_limit: Duration,
    client_closed_callback: Box<dyn Fn()>,
    message_handler: Arc<&'static (dyn Fn(String) -> Option<String> + Sync + Send)>,
}

impl ReactiveWebsocketState {
    pub fn new(config: ReactiveWebsocketConfig) -> Self {
        Self {
            active_clients: AtomicUsize::new(0),
            rejection_counter: AtomicUsize::new(0),
            config,
        }
    }
}

impl ReactiveActor {
    fn new(config: &'static ReactiveWebsocketConfig, client_closed_callback: Box<dyn Fn()>) -> Self {
        Self {
            rapid_request_rejection_enabled: config.rapid_request_limit.is_none(),
            last_request_stopwatch: Instant::now(),
            rapid_request_limit: if config.rapid_request_limit.is_none() {
                Duration::from_secs(0)
            } else {
                config.rapid_request_limit.unwrap()
            },
            client_closed_callback,
            message_handler: config.message_handler.clone(),
        }
    }
}

impl ActixActor for ReactiveActor {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        context.set_mailbox_capacity(ACTOR_MAILBOX_CAPACITY);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        (*self.client_closed_callback)();
        Running::Stop
    }
}

impl StreamHandler<WsMessage, WsProtocolError> for ReactiveActor {
    fn handle(&mut self, payload: WsMessage, context: &mut Self::Context) {
        if self.rapid_request_rejection_enabled {
            if self.last_request_stopwatch.elapsed() < self.rapid_request_limit {
                context.stop();
                return;
            }
            self.last_request_stopwatch = Instant::now();
        }
        match payload {
            WsMessage::Close(_) => context.stop(),
            WsMessage::Ping(ping_payload) => context.pong(&ping_payload),
            WsMessage::Text(text) => {
                let handler_clone = self.message_handler.clone();
                if let Some(response_string) = handler_clone(text) {
                    context.text(response_string)
                }
            }
            _ => (),
        }
    }
}

fn reject_unmapped_handler(
    shared_state: ActixData<Arc<&'static ReactiveWebsocketState>>,
) -> Box<dyn Future<Item = HttpResponse, Error = HttpError>> {
    shared_state.rejection_counter.fetch_add(1, Ordering::Relaxed);
    debug!(
        "Rejected counter increased to {}",
        shared_state.rejection_counter.load(Ordering::Relaxed)
    );
    let mut error = Vec::default();
    error.push(NOTFOUND_MESSAGE.to_owned());
    let response_data = CommonResponse {
        error,
        result: HashMap::new(),
    };
    Box::new(ok::<_, HttpError>(
        HttpResponse::NotFound().body(serde_json::to_string(&response_data).unwrap()),
    ))
}

fn ws_upgrader(
    shared_state: ActixData<Arc<&'static ReactiveWebsocketState>>,
    request: HttpRequest,
    stream: Payload,
) -> Result<HttpResponse, HttpError> {
    let ReactiveWebsocketState {
        config, active_clients, ..
    } = shared_state.get_ref().as_ref();
    config.auth.validate(&request)?;
    let upgrade_result = ws_start(
        ReactiveActor::new(
            &config,
            Box::new(move || {
                let active_clients = active_clients.fetch_sub(1, Ordering::Relaxed);
                info!(
                    "Client connection closed, current active client is {}",
                    active_clients - 1
                );
            }),
        ),
        &request,
        stream,
    );
    if upgrade_result.is_ok() {
        let active_clients = shared_state.active_clients.fetch_add(1, Ordering::Relaxed);
        info!(
            "Client connection successful, current active client is {}",
            active_clients + 1
        );
    }
    upgrade_result
}

pub fn run_reactive_websocket_service(state: Arc<&'static ReactiveWebsocketState>) -> IOResult<()> {
    let ReactiveWebsocketConfig {
        binding_url,
        binding_path,
        max_clients,
        ..
    } = &state.config;
    let shared_data = ActixData::new(state);
    ActixHttpServer::new(move || {
        ActixApp::new()
            .register_data(shared_data.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource(&binding_path).route(web::get().to(ws_upgrader)))
            .default_service(web::route().to_async(reject_unmapped_handler))
    })
    .maxconn(*max_clients)
    .shutdown_timeout(1)
    .bind(binding_url)
    .unwrap()
    .run()
}
