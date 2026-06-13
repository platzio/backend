use actix::prelude::*;
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use platz_auth::ApiIdentity;
use platz_db::{AccessScope, DbEvent, DbEventData, DbEventOperation, db};
use std::time::Duration;
use tokio::sync::{broadcast::error::RecvError, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, warn};

/// Subprotocol used to carry the access token. Browsers cannot set an
/// `Authorization` header on a WebSocket, so the client authenticates by
/// connecting with `new WebSocket(url, [WS_AUTH_PROTOCOL, <access_token>])`,
/// which the browser sends as the `Sec-WebSocket-Protocol` request header.
const WS_AUTH_PROTOCOL: &str = "platz-auth-bearer";

/// A websocket connection that streams database change events to a single
/// authenticated client. The events have already been filtered to the client's
/// [`AccessScope`] by a per-connection task before reaching the actor.
struct DbEventsWs {
    /// Stream of authorized events. Taken in `started` to feed the actor.
    events: Option<UnboundedReceiverStream<DbEvent>>,
}

impl Actor for DbEventsWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(events) = self.events.take() {
            ctx.add_stream(events);
        }
        ctx.run_interval(Duration::from_secs(30), Self::keepalive);
    }
}

impl DbEventsWs {
    fn keepalive(&mut self, ctx: &mut <Self as Actor>::Context) {
        ctx.ping(&[]);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for DbEventsWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

/// Authorized database events forwarded by the per-connection filter task.
impl StreamHandler<DbEvent> for DbEventsWs {
    fn handle(&mut self, event: DbEvent, ctx: &mut Self::Context) {
        match serde_json::to_string(&event) {
            Ok(payload) => ctx.text(payload),
            Err(err) => {
                error!("Error serializing DB event for websocket: {err}");
                ctx.stop();
            }
        }
    }
}

/// Extract the access token from the `Sec-WebSocket-Protocol` header. The header
/// carries the auth subprotocol name followed by the token itself.
fn extract_token(req: &HttpRequest) -> Option<String> {
    let header = req
        .headers()
        .get("Sec-WebSocket-Protocol")?
        .to_str()
        .ok()?;
    let mut parts = header.split(',').map(str::trim);
    if parts.next()? != WS_AUTH_PROTOCOL {
        return None;
    }
    parts.next().filter(|token| !token.is_empty()).map(String::from)
}

async fn connect_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    // Authenticate the connection. Unlike the firehose this replaced, an
    // unauthenticated client can no longer connect and receive events.
    let token = extract_token(&req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing websocket access token"))?;
    let identity = ApiIdentity::from_access_token(&token).await?;
    let scope = AccessScope::for_identity(identity.inner())
        .await
        .map_err(|err| actix_web::error::ErrorServiceUnavailable(err.to_string()))?;

    let db = db().map_err(|err| actix_web::error::ErrorServiceUnavailable(err.to_string()))?;
    let mut events = db.subscribe_to_events();

    // The actor runs synchronously and cannot await the per-event authorization
    // queries. A dedicated task does that work, forwarding only the events this
    // identity may see. Decisions are awaited sequentially so event ordering
    // (e.g. UPDATE before DELETE for the same row) is preserved.
    let (tx, rx) = mpsc::unbounded_channel::<DbEvent>();
    actix_web::rt::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => match scope.can_receive_event(&event).await {
                    Ok(true) => {
                        if tx.send(event).is_err() {
                            // Receiver (the websocket actor) is gone.
                            break;
                        }
                    }
                    Ok(false) => {}
                    Err(err) => error!("Error authorizing websocket event: {err}"),
                },
                Err(RecvError::Lagged(skipped)) => {
                    warn!("Websocket event listener lagged, skipped {skipped} events");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    let actor = DbEventsWs {
        events: Some(UnboundedReceiverStream::new(rx)),
    };
    // Echo the auth subprotocol back so the browser's WebSocket handshake
    // succeeds.
    ws::WsResponseBuilder::new(actor, &req, stream)
        .protocols(&[WS_AUTH_PROTOCOL])
        .start()
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(connect_ws));
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Events",
        description = "Events sent through the Websocket.",
    )),
    components(schemas(DbEvent, DbEventOperation, DbEventData)),
)]
pub(super) struct OpenApi;
