use std::collections::{HashMap};
use rand::{self, rngs::ThreadRng, Rng};

use actix::prelude::*;

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

    fn send_message(&self, message: &str, skip_id: usize) {
        for (id,rcp) in &self.sessions {
            if *id != skip_id {
                let _ = rcp.do_send(Message(message.to_owned()));
            }
        }
    }

    pub fn gen_random_handle(&mut self) -> String {
        let rand_part = self.rand_gen.gen::<u32>();
        let mut handle = String::from("guest");
        handle.push_str(&rand_part.to_string());

        handle
    }
}
impl Actor for ChatServ {
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct Message(pub String);

#[derive(Message, Serialize)]
pub struct ClientMessage {
    pub id: usize,
    pub user: String,
    pub text: String,
}

#[derive(Message)]
#[rtype("Result<(usize, String), std::io::Error>")]
pub struct Connect {
    pub address: Recipient<Message>,
    pub req_handle: Option<String>,
}

#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
    pub handle: String,
}


impl Handler<Connect> for ChatServ {
    type Result = Result<(usize, String), std::io::Error>;

    fn handle(&mut self, message: Connect, _: &mut Context<Self>)
              -> Result<(usize, String), std::io::Error> {
        let id = self.rand_gen.gen::<usize>();
        self.sessions.insert(id, message.address);

        let handle = match message.req_handle {
            Some(s) => s,
            _ => self.gen_random_handle()
        };
        self.usernames.insert(id, handle.clone());

        let out = ClientMessage {
            id: 0,
            user: String::from("Host"),
            text: format!("{} connected!", handle),
        };
        self.send_message(serde_json::to_string(&out).unwrap().as_str(), 0);

        return Ok((id, handle))
    }
}


impl Handler<Disconnect> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&message.id);
        self.usernames.remove(&message.id);

        println!("Successfully disconnected!");

        let out = ClientMessage {
            id: 0,
            user: String::from("Host"),
            text: format!("{} disconnected!", message.handle),
        };
        self.send_message(serde_json::to_string(&out).unwrap().as_str(), 0);
    }
}

impl Handler<ClientMessage> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: ClientMessage, _: &mut Context<Self>) {
        let msg = serde_json::to_string(&message);
        self.send_message(msg.unwrap().as_str(), message.id);
    }
}
