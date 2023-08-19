use std::sync::RwLock;
use actix::{Actor, ActorContext, AsyncContext, Handler, StreamHandler};
use actix_web::{get, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use crate::appstate::{AppState, UpdateMessage};

pub struct PlaceWebSocketConnection{
    appstate: web::Data<RwLock<AppState>>,
}

impl Actor for PlaceWebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        match self.appstate.write() {
            Ok(appstate) => {
                appstate.add_session(ctx.address());
            }
            Err(err) => {
                println!("Error writing to app state: {}", err);
                ctx.stop();
            }
        }
    }
}

impl Handler<UpdateMessage> for PlaceWebSocketConnection {
    type Result = ();

    fn handle(&mut self, msg: UpdateMessage, ctx: &mut Self::Context) {
        let text = match serde_json::to_string(&msg) {
            Ok(text) => text,
            Err(_) => {
                ctx.text("Error serializing update message");
                return;
            }
        };

        ctx.text(text);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PlaceWebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[get("/api/ws")]
async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<RwLock<AppState>>,
) -> HttpResponse {
    match ws::start(
        PlaceWebSocketConnection {
            appstate: data,
        },
        &req,
        stream,
    ) {
        Ok(response) => response,
        Err(error) => {
            println!("Error starting websocket: {}", error);
            HttpResponse::InternalServerError().body("error")
        }
    }
}