use crate::actix::Actor as ActixActor;
use crate::actix::ActorContext;
use crate::actix::Addr;
use crate::actix::AsyncContext;
use crate::actix::Handler;
use crate::actix::Message;
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
use crate::common_types::CommonResponse;
use crate::crossbeam_channel::unbounded as create_mpmc_channel;
use crate::crossbeam_channel::SendError;
use crate::crossbeam_channel::Sender;
use crate::crossbeam_utils::thread as scoped_thread;
use crate::debug;
use crate::error;
use crate::futures::executor::spawn as spawn_future;
use crate::futures::future::ok;
use crate::futures::Future;
use crate::futures_locks::RwLock as AsyncRwLock;
use crate::futures_locks::RwLockWriteGuard;
use crate::info;
use crate::ACTOR_MAILBOX_CAPACITY;
use crate::NOTFOUND_MESSAGE;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

pub type StaticStateArc = Arc<&'static PubsubWebsocketState>;
pub type SendBroadcastFunction = Arc<dyn Fn(String) + Send + Sync>;
type AsyncHttpResult = dyn Future<Item = HttpResponse, Error = HttpError>;
type SyncHttpResult = Result<HttpResponse, HttpError>;
type SubscribeResult = Result<(), SendError<BroadcastSubscribeSignal>>;
type ClientAddress = Addr<PubsubBroadcastActor>;
type ClientsDictionary = HashMap<ClientAddress, ()>;

pub struct PubsubWebsocketConfig {
    pub binding_url: String,
    pub binding_path: String,
    pub max_clients: usize,
    pub client_timeout: Duration,
    pub rapid_request_limit: Duration,
}

pub struct PubsubWebsocketState {
    pub active_clients: AtomicUsize,
    pub rejection_counter: AtomicUsize,
    pub config: PubsubWebsocketConfig,
    subscribe_signaler: RwLock<Option<BroadcastSubscriber>>,
}

pub(crate) struct PubsubBroadcastActor {
    last_request_stopwatch: Instant,
    rapid_request_limit: Duration,
    pubsub_signaler: Cell<BroadcastSubscriber>,
    client_closed_callback: Box<dyn Fn()>,
}

impl PubsubWebsocketState {
    pub fn new(config: PubsubWebsocketConfig) -> Self {
        Self {
            active_clients: AtomicUsize::new(0),
            rejection_counter: AtomicUsize::new(0),
            config,
            subscribe_signaler: RwLock::new(None),
        }
    }

    fn set_subscriber(&self, pubsub_signaler: BroadcastSubscriber) {
        let mut write_guard = self.subscribe_signaler.write().unwrap();
        *write_guard = Some(pubsub_signaler);
    }
}

impl PubsubBroadcastActor {
    fn new(
        config: &'static PubsubWebsocketConfig,
        pubsub_signaler: BroadcastSubscriber,
        client_closed_callback: Box<dyn Fn()>,
    ) -> Self {
        Self {
            last_request_stopwatch: Instant::now(),
            rapid_request_limit: config.rapid_request_limit,
            pubsub_signaler: Cell::new(pubsub_signaler),
            client_closed_callback,
        }
    }
}

impl ActixActor for PubsubBroadcastActor {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        context.set_mailbox_capacity(ACTOR_MAILBOX_CAPACITY);
        let _ = self.pubsub_signaler.get_mut().subscribe(context.address());
    }

    fn stopping(&mut self, context: &mut Self::Context) -> Running {
        let subscriber = self.pubsub_signaler.take();
        let _ = subscriber.unsubscribe(context.address());
        (*self.client_closed_callback)();
        Running::Stop
    }
}

#[derive(Clone, Message)]
pub struct BroadcastMessage(String);

impl Handler<BroadcastMessage> for PubsubBroadcastActor {
    type Result = ();

    fn handle(&mut self, message: BroadcastMessage, context: &mut Self::Context) {
        context.text(message.0);
    }
}

impl StreamHandler<WsMessage, WsProtocolError> for PubsubBroadcastActor {
    fn handle(&mut self, payload: WsMessage, context: &mut Self::Context) {
        if self.last_request_stopwatch.elapsed() < self.rapid_request_limit {
            context.stop();
            return;
        }
        self.last_request_stopwatch = Instant::now();
        match payload {
            WsMessage::Close(_) => context.stop(),
            WsMessage::Ping(ping_payload) => context.pong(&ping_payload),
            WsMessage::Text(text) => {
                if text.len() < 4 {
                    return;
                }
                if let "ping" = &text.to_lowercase()[0..4] {
                    context.text("pong")
                }
            }
            _ => (),
        }
    }
}

pub(crate) enum BroadcastSubscribeSignal {
    Subscribe(ClientAddress),
    Unsubcribe(ClientAddress),
}

#[derive(Clone)]
pub(crate) struct BroadcastSubscriber {
    subscribe_signaler: Option<Sender<BroadcastSubscribeSignal>>,
}

impl Default for BroadcastSubscriber {
    fn default() -> Self {
        Self {
            subscribe_signaler: None,
        }
    }
}

impl BroadcastSubscriber {
    fn new(subscribe_signaler: Sender<BroadcastSubscribeSignal>) -> Self {
        Self {
            subscribe_signaler: Some(subscribe_signaler),
        }
    }

    fn subscribe(&self, client_identity: ClientAddress) -> SubscribeResult {
        if self.subscribe_signaler.is_none() {
            panic!("The websocket client is trying to register itself without a subscriber!")
        }
        self.subscribe_signaler
            .as_ref()
            .unwrap()
            .send(BroadcastSubscribeSignal::Subscribe(client_identity))
    }

    fn unsubscribe(self, client_identity: ClientAddress) -> SubscribeResult {
        if self.subscribe_signaler.is_none() {
            panic!("The websocket client is trying to register itself without a subscriber!")
        }
        self.subscribe_signaler
            .unwrap()
            .send(BroadcastSubscribeSignal::Unsubcribe(client_identity))
    }
}

fn reject_unmapped_handler(shared_state: ActixData<StaticStateArc>) -> Box<AsyncHttpResult> {
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

fn ws_upgrader(shared_state: ActixData<StaticStateArc>, request: HttpRequest, stream: Payload) -> SyncHttpResult {
    let PubsubWebsocketState { active_clients, .. } = shared_state.get_ref().as_ref();
    let onclose_callback = Box::new(move || {
        let active_clients = active_clients.fetch_sub(1, Ordering::Relaxed);
        info!(
            "Client connection closed, current active client is {}",
            active_clients - 1
        );
    });
    let subscribe_signaler_guard = shared_state.subscribe_signaler.read().unwrap();
    let cloned_subscribe_signaler = subscribe_signaler_guard.as_ref().unwrap().clone();
    let pubsub_broadcast_actor = PubsubBroadcastActor::new(
        &shared_state.get_ref().config,
        cloned_subscribe_signaler,
        onclose_callback,
    );
    let upgrade_result = ws_start(pubsub_broadcast_actor, &request, stream);
    match upgrade_result {
        Ok(ok_result) => {
            let active_clients = shared_state.active_clients.fetch_add(1, Ordering::Relaxed);
            info!(
                "Client connection successful, current active client is {}",
                active_clients + 1
            );
            Ok(ok_result)
        }
        Err(error_result) => {
            error!("{:?}", error_result);
            Err(error_result)
        }
    }
}

pub fn run_pubsub_websocket_service(state: StaticStateArc, send_broadcast_fn: Sender<SendBroadcastFunction>) {
    // Section broadcast pubsub threads
    let shutdown_signal = AtomicBool::new(false);
    let (subscribe_signaler, subscribe_listener) = create_mpmc_channel::<BroadcastSubscribeSignal>();
    state.set_subscriber(BroadcastSubscriber::new(subscribe_signaler));
    let (publisher_sender, publisher_receiver) = create_mpmc_channel::<BroadcastMessage>();
    let broadcaster = move |message: String| {
        let _ = publisher_sender.send(BroadcastMessage(message));
    };
    let _ = send_broadcast_fn.send(Arc::new(broadcaster));
    info!("Broadcaster callback sent, running Pubsub Broadcast thread...");
    let max_clients = state.config.max_clients;
    let subscribers: ClientsDictionary = HashMap::with_capacity(max_clients);
    let rw_lock_registrar = AsyncRwLock::new(subscribers);
    let rw_lock_publisher = rw_lock_registrar.clone();
    let no_message_timeout = Duration::from_secs(1);
    let client_timeout = state.config.client_timeout.as_millis() as u64;
    scoped_thread::scope(|s| {
        // Subscribe/Unsubscribe registration thread
        s.spawn(|_| {
            let rw_lock = rw_lock_registrar;
            let no_message_timeout = Duration::from_secs(1);
            let insert_client_func = |mut clients: RwLockWriteGuard<ClientsDictionary>,
                                      client_address: ClientAddress| {
                clients.entry(client_address).or_insert(());
                info!("A Client just subscribed, current client count is {}", clients.len());
            };
            let remove_client_func = |mut clients: RwLockWriteGuard<ClientsDictionary>,
                                      client_address: ClientAddress| {
                clients.remove(&client_address).unwrap();
                info!("A Client just unsubscribed, current client count is {}", clients.len());
            };
            loop {
                if shutdown_signal.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(subscribe_signal) = subscribe_listener.recv_timeout(no_message_timeout) {
                    match subscribe_signal {
                        BroadcastSubscribeSignal::Subscribe(client_addr) => {
                            let _ = rw_lock
                                .write()
                                .map(|clients| insert_client_func(clients, client_addr))
                                .wait();
                        }
                        BroadcastSubscribeSignal::Unsubcribe(client_addr) => {
                            let _ = rw_lock
                                .write()
                                .map(|clients| remove_client_func(clients, client_addr))
                                .wait();
                        }
                    }
                }
            }
        });
        // Message broadcast thread
        s.spawn(|_| {
            let rw_lock = rw_lock_publisher;
            loop {
                if shutdown_signal.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(message) = publisher_receiver.recv_timeout(no_message_timeout) {
                    let async_read = rw_lock.read().map(|clients| {
                        for (client, _) in clients.iter() {
                            client.do_send(message.clone());
                        }
                    });
                    let _ = spawn_future(async_read).wait_future();
                }
            }
        });
        // Websocket server thread
        s.spawn(|_| {
            let PubsubWebsocketConfig {
                binding_url,
                binding_path,
                ..
            } = &state.config;
            let shared_data = ActixData::new(state);
            info!("Running Actix Websocket server...");
            let _ = ActixHttpServer::new(move || {
                ActixApp::new()
                    .register_data(shared_data.clone())
                    .wrap(middleware::Logger::default())
                    .service(web::resource(&binding_path).route(web::get().to(ws_upgrader)))
                    .default_service(web::route().to_async(reject_unmapped_handler))
            })
            .maxconn(max_clients)
            .client_timeout(client_timeout)
            .client_shutdown(client_timeout)
            .shutdown_timeout(1)
            .bind(binding_url)
            .unwrap()
            .run();
            shutdown_signal.store(true, Ordering::Relaxed);
        });
    })
    .unwrap();
}
