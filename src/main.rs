extern crate serde_json;
#[macro_use] extern crate serde_derive;

extern crate actix;
extern crate actix_web;
extern crate futures;

use std::time::{Instant, Duration};

use actix::*;
use actix_web::{fs, http, ws, server as httpserv,
                App, Query, HttpRequest, HttpResponse, Error};

mod chatserv;

struct ChatSession {
    id: usize,
    username: Option<String>,
    hb: Instant,
}
impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self, ChatSessionState>;

    fn started(&mut self, context: &mut Self::Context) {
        self.heartbeat(context);
        context.state().address
            .send(chatserv::Connect {
                address: context.address().recipient(),
                req_handle: self.username.clone(),
            })
            .into_actor(self)
            .then(|response, act, ctx| {
                match response {
                    Ok(new_user) => {
                        match new_user {
                            Ok(user_info) => {
                                let (new_id, new_username) = user_info;
                                act.id = new_id;
                                act.username = Some(new_username);
                                println!("Connected user: {:?}", act.username);
                            }
                            _ => ctx.stop()
                        }
                    }
                    _ => ctx.stop()
                }
                fut::ok(())
            })
            .wait(context);
    }

    fn stopping(&mut self, context: &mut Self::Context) -> Running {
        let username = self.username.clone().unwrap();
        println!("Disconnecting user: {:?}", username);
        context.state().address.do_send(chatserv::Disconnect { id: self.id, handle: username });
        Running::Stop
    }
}
impl StreamHandler<ws::Message, ws::ProtocolError> for ChatSession {
    fn handle(&mut self, message: ws::Message, context: &mut Self::Context) {
        match message {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                context.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(txt) => {
                let username = self.username.clone().unwrap_or(String::from("MysteryGuest"));
                context.state().address
                    .do_send(chatserv::ClientMessage { id: self.id,
                                                       user: username,
                                                       text: txt.trim().to_string() });
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

const HB_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
impl ChatSession {
    fn heartbeat(&self, context: &mut ws::WebsocketContext<Self, ChatSessionState>) {
        context.run_interval(HB_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Client heartbeat failed; disconnecting {:?}", act.username);
                ctx.stop();
            }
            else { ctx.ping("U up?"); }
        });
    }
}

struct ChatSessionState {
    address: Addr<chatserv::ChatServ>,
}

#[derive(Deserialize)]
struct HandleRequest {
    req_handle: String,
}
fn start_registered(req: HttpRequest<ChatSessionState>, query: Query<HandleRequest>)
                    -> Result<HttpResponse, Error> {
    let req_handle = &query.req_handle;
    ws::start(&req,
              ChatSession { id: 0,
                            username: Some(req_handle.clone()),
                            hb: Instant::now(),
              })
}

// do handshake, start actor
fn start_guest(req: &HttpRequest<ChatSessionState>) -> Result<HttpResponse, Error> {
    ws::start(req,
              ChatSession { id: 0,
                            username: None,
                            hb: Instant::now(),
              })
}

fn main() {
    const HOST: &str = "localhost";
    const PORT: u16 = 10000;

    let sys = actix::System::new("rusty-chat");

    // starts chat server on a separate thread
    let server = Arbiter::start(|_| chatserv::ChatServ::new());

    httpserv::new(move || {
        let state = ChatSessionState { address: server.clone() };

        App::with_state(state)
            // redirect to chat.html
            .resource("/", |r| r.method(http::Method::GET).f(|_| {
                HttpResponse::Found()
                    .header("LOCATION", "/app/chat.html")
                    .finish()
            }))
            .resource("/register", |r| r.route().with(start_registered))
            .resource("/guest", |r| r.route().f(start_guest))
            // serve static resources
            .handler("/app/", fs::StaticFiles::new("app/").unwrap())
    }).bind(format!("{}:{}", HOST, PORT)).unwrap().start();

    println!("Started server at: {}:{}", HOST, PORT);

    let _ = sys.run();
}
