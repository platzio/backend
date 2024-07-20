use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use platz_db::{db_events, DbEvent};
use std::time::Duration;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tracing::error;

#[derive(Default)]
struct DbEventsWs {}

impl Actor for DbEventsWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let stream = BroadcastStream::new(db_events());
        ctx.add_stream(stream);
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

impl StreamHandler<Result<DbEvent, BroadcastStreamRecvError>> for DbEventsWs {
    fn handle(
        &mut self,
        event: Result<DbEvent, BroadcastStreamRecvError>,
        ctx: &mut Self::Context,
    ) {
        match event {
            Ok(event) => ctx.text(serde_json::to_string(&event).unwrap()),
            Err(err) => {
                error!("Error in websocket stream handler: {:?}", err);
                ctx.stop();
            }
        }
    }
}

async fn connect_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(DbEventsWs::default(), &req, stream)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(connect_ws));
}
