use std::collections::{HashMap};
use rand::{self, rngs::ThreadRng, Rng};

use actix::prelude::*;

// coordinating server, just talks to client sessions
pub struct ChatServ {
    sessions: HashMap<usize, Recipient<Message>>,
    usernames: HashMap<usize, String>,
    rand_gen: ThreadRng,
}
impl ChatServ {
    pub fn new() -> ChatServ {
        ChatServ {
            sessions: HashMap::new(),
            usernames: HashMap::new(),
            rand_gen: rand::thread_rng(),
        }
    }

    fn send_message(&self, message: ClientMessage, skip_id: usize) {
        let parsed_message = serde_json::to_string(&message).unwrap();
        for (id,rcp) in &self.sessions {
            if *id != skip_id {
                let _ = rcp.do_send(Message(parsed_message.as_str().to_owned()));
            }
        }
    }

    pub fn gen_random_username(&mut self) -> String {
        let rand_part = self.rand_gen.gen::<u32>();
        let mut username = String::from("guest");
        username.push_str(&rand_part.to_string());

        username
    }
}
impl Actor for ChatServ {
    // only needs a simple context since it's not talking to WS clients
    type Context = Context<Self>;
}

// Message types

#[derive(Message)]
pub struct Message(pub String);

#[derive(Message, Serialize)]
pub struct ClientMessage {
    pub id: usize,
    pub user: String,
    pub text: String,
    pub to_users: HashMap<usize, String>
}

#[derive(Message)]
#[rtype("Result<(usize, String), std::io::Error>")]
pub struct Connect {
    pub address: Recipient<Message>,
    pub req_username: Option<String>,
}

#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
    pub username: Option<String>,
}

// Handlers for message types

impl Handler<Connect> for ChatServ {
    type Result = Result<(usize, String), std::io::Error>;

    fn handle(&mut self, message: Connect, _: &mut Context<Self>)
              -> Result<(usize, String), std::io::Error> {

        let id = self.rand_gen.gen::<usize>();
        self.sessions.insert(id, message.address);

        let username = match message.req_username {
            Some(s) => s,
            _ => self.gen_random_username()
        };
        self.usernames.insert(id, username.clone());

        // notify other clients that someone connected
        let out = ClientMessage {
            id: 0,
            user: String::from("Host"),
            text: format!("{} connected!", username),
            to_users: self.usernames.clone(),
        };

        self.send_message(out, 0);

        return Ok((id, username))
    }
}


impl Handler<Disconnect> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&message.id);
        self.usernames.remove(&message.id);

        println!("Successfully disconnected!");

        /* only notify existing users of disconnection
         if username had been successfully initialized */
        match message.username {
            Some(username) => {
                let out = ClientMessage {
                    id: 0,
                    user: String::from("Host"),
                    text: format!("{} disconnected!", username),
                    to_users: self.usernames.clone(),
                };
                self.send_message(out, 0);
            }
            _ => {}
        }
    }
}

impl Handler<ClientMessage> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: ClientMessage, _: &mut Context<Self>) {

        let skip_id = message.id;

        /* client doesn't know which users are still connected
         so we append that to the message before sending */
        let mut updated_msg = message;
        updated_msg.to_users = self.usernames.clone();

        self.send_message(updated_msg, skip_id);
    }
}
