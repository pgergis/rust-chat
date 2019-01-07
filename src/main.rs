extern crate actix;
extern crate actix_web;
extern crate futures;

use actix::*;
use actix_web::{fs, http, ws, server as httpserv, App, HttpRequest, HttpResponse, Error};

mod chatserv;

struct ChatSession {
    id: usize,
}
impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self, ChatSessionState>;

    fn started(&mut self, context: &mut Self::Context) {
        context.state().address.send(chatserv::Connect { address: context.address().recipient() }).into_actor(self)
            .then(|response, act, ctx| {
                match response {
                    Ok(response) => act.id = response,
                    _ => ctx.stop()
                }
                fut::ok(())
            })
            .wait(context);
    }

    fn stopping(&mut self, context: &mut Self::Context) -> Running {
        context.state().address.do_send(chatserv::Disconnect { id: self.id });
        Running::Stop
    }
}
impl StreamHandler<ws::Message, ws::ProtocolError> for ChatSession {
    fn handle(&mut self, message: ws::Message, context: &mut Self::Context) {
        match message {
            ws::Message::Ping(mes) => {
                context.pong(&mes);
            }
            ws::Message::Pong(_) => {
                println!("Ponged!");
            }
            ws::Message::Text(text) => {
                let mes = text.trim();
                context.state().address.do_send(chatserv::ClientMessage { id: self.id, msg: mes.to_string() });
            }
            ws::Message::Binary(_) => {
                println!("Don't support binary!");
            }
            ws::Message::Close(_) => {
                context.close(None);
            }
        }
    }
}
impl Handler<chatserv::Message> for ChatSession {
    type Result = ();

    fn handle(&mut self, message: chatserv::Message, context: &mut Self::Context) {
        context.text(message.0);
    }
}

struct ChatSessionState {
    address: Addr<chatserv::ChatServ>,
}

// do handshake, start actor
fn initialize(req: &HttpRequest<ChatSessionState>) -> Result<HttpResponse, Error> {
    ws::start(req, ChatSession { id: 0 })
}

fn main() {
    let sys = actix::System::new("rusty-chat");

    // starts chat server on a separate thread
    let server = Arbiter::start(|_| chatserv::ChatServ::new());

    httpserv::new(move || {
        let state = ChatSessionState { address: server.clone() };

        App::with_state(state)
            // redirect to chat.html
            .resource("/", |r| r.method(http::Method::GET).f(|_| {
                HttpResponse::Found()
                    .header("LOCATION", "/elm/chat.html")
                    .finish()
            }))
            .resource("/ws/", |r| r.route().f(initialize))
            // static resources
            .handler("/elm/", fs::StaticFiles::new("elm/").unwrap())
    }).bind("localhost:8000").unwrap().start();

    println!("Started server at: localhost:8000");

    let _ = sys.run();
}
