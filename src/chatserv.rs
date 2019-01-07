use std::collections::{HashMap};
use rand::{self, rngs::ThreadRng, Rng};

use actix::prelude::*;

pub struct ChatServ {
    sessions: HashMap<usize, Recipient<Message>>,
    rand_gen: ThreadRng,
}
impl ChatServ {
    pub fn new() -> ChatServ {
        ChatServ {
            sessions: HashMap::new(),
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
}
impl Actor for ChatServ {
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub address: Recipient<Message>,
}
impl Handler<Connect> for ChatServ {
    type Result = usize;

    fn handle(&mut self, message: Connect, _: &mut Context<Self>) -> usize {
        println!("Someone connected!");
        self.send_message("Someone connected!", 0);

        let id = self.rand_gen.gen::<usize>();
        self.sessions.insert(id, message.address);

        return id
    }
}


#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
}
impl Handler<Disconnect> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnect!");

        self.sessions.remove(&message.id);

        self.send_message("Someone disconnected!", 0);
    }
}

#[derive(Message)]
pub struct ClientMessage {
    pub id: usize,
    pub msg: String,
}
impl Handler<ClientMessage> for ChatServ {
    type Result = ();

    fn handle(&mut self, message: ClientMessage, _: &mut Context<Self>) {
        self.send_message(message.msg.as_str(), message.id);
    }
}
