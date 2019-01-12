extern crate serde_json;
#[macro_use] extern crate serde_derive;

extern crate actix;
extern crate actix_web;
extern crate futures;

use std::collections::{HashMap};
use std::time::{Instant, Duration};

use actix::*;
use actix_web::{fs, http, ws, server as httpserv,
                App, Query, HttpRequest, HttpResponse, Error};

mod chatserv;

struct ChatSession {
    id: usize,
    username: Option<String>,
    hb: Instant, // last time heartbeat was checked
    known_others: HashMap<usize, String>,
}
impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self, ChatSessionState>;

    // runs when Actor spins up
    fn started(&mut self, context: &mut Self::Context) {
        self.heartbeat(context);
        context.state().address
            // notify coordinating server of connection
            .send(chatserv::Connect {
                address: context.address().recipient(),
                req_username: self.username.clone(),
            })
            .into_actor(self)
            .then(|response, act, ctx| {
                match response {
                    // the coordinating server responds
                    Ok(new_user) => {
                        match new_user {
                            // the coordinating server accepts the requested username
                            Ok(user_info) => {
                                let (new_id, new_username) = user_info;
                                act.id = new_id;
                                act.username = Some(new_username);
                                println!("Connected user: {:?}", act.username);
                            }
                            Err(e) => {
                                println!("{:?}", e);
                                ctx.stop()
                            }
                        }
                    }
                    _ => ctx.stop()
                }
                fut::ok(())
            })
            .wait(context);
    }

    // runs when Actor is spinning down
    fn stopping(&mut self, context: &mut Self::Context) -> Running {
        let username = self.username.clone();
        println!("Disconnecting user: {:?}", username);

        // notify connecting server of disconnection
        context.state().address.do_send(chatserv::Disconnect { id: self.id, username: username });
        Running::Stop
    }
}

// handling wc protocol messages
impl StreamHandler<ws::Message, ws::ProtocolError> for ChatSession {
    fn handle(&mut self, message: ws::Message, context: &mut Self::Context) {
        match message {
            // per protocol: must respond to pings with the body received
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                context.pong(&msg);
            }
            // update heartbeat; no reply expected
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(txt) => {
                let username = self.username.clone()
                    // Unwrap should never fail: server always assigns random guest names
                    .unwrap_or(String::from("MysteryGuest"));
                context.state().address
                    // package text message into format server can handle, and forward along
                    .do_send(chatserv::ClientMessage { id: self.id,
                                                       user: username,
                                                       text: txt.trim().to_string(),
                                                       to_users: self.known_others.clone(),
                    });
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

// only message received from coordinating server should be text
impl Handler<chatserv::Message> for ChatSession {
    type Result = ();

    fn handle(&mut self, message: chatserv::Message, context: &mut Self::Context) {
        context.text(message.0);
    }
}

// helper functions for client actor
const HB_INTERVAL: Duration = Duration::from_secs(5); // keep-alive interval
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
impl ChatSession {

    // checks that client is still active
    fn heartbeat(&self, context: &mut ws::WebsocketContext<Self, ChatSessionState>) {
        context.run_interval(HB_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Client heartbeat failed; disconnecting {:?}", act.username);
                ctx.stop();
            }
            // keep-alive
            else { ctx.ping("U up?"); }
        });
    }
}

struct ChatSessionState {
    address: Addr<chatserv::ChatServ>,
}


// Two separate functions for connecting a client: for guests, and for registered w/ handles
// both initiate handshake and spin up an actor (session) for the client
#[derive(Deserialize)]
struct UsernameRequest {
    req_username: String,
}
// accepts username request via URL query params
fn start_registered(req: HttpRequest<ChatSessionState>, query: Query<UsernameRequest>)
                    -> Result<HttpResponse, Error> {
    let req_username = &query.req_username;
    ws::start(&req,
              ChatSession { id: 0,
                            username: Some(req_username.clone()),
                            hb: Instant::now(),
                            known_others: HashMap::new(),
              })
}

fn start_guest(req: &HttpRequest<ChatSessionState>) -> Result<HttpResponse, Error> {
    ws::start(req,
              ChatSession { id: 0,
                            username: None,
                            hb: Instant::now(),
                            known_others: HashMap::new(),
              })
}

fn main() {
    const HOST: &str = "localhost";
    const PORT: u16 = 8080;

    let sys = actix::System::new("rusty-chat");

    // starts chat server (coordinates client sessions) on a separate thread
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
            // serve front-end
            .handler("/app/", fs::StaticFiles::new("app/").unwrap())
    }).bind(format!("{}:{}", HOST, PORT)).unwrap().start();

    println!("Started server at: {}:{}", HOST, PORT);

    let _ = sys.run();
}
