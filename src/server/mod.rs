extern crate http_muncher;
extern crate mio;
extern crate rustc_serialize;
extern crate sha1;

use std::collections::HashMap;

use mio::*;
use mio::tcp::*;

use crate::client::WebSocketClient;

pub struct WebSocketServer {
    pub socket: TcpListener,
    clients: HashMap<Token, WebSocketClient>,
    pub token_counter: usize
}

impl WebSocketServer {
    pub fn new(socket: TcpListener) -> WebSocketServer {
        WebSocketServer {
            socket: socket,
            token_counter: 1,
            clients: HashMap::new()
        }
    }
}

const SERVER_TOKEN: Token = Token(0);

impl Handler for WebSocketServer {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<WebSocketServer>,
             token: Token,
             events: EventSet) {
        if events.is_readable() {
            match token {
                SERVER_TOKEN => {
                    let client_socket = match self.socket.accept() {
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        },
                        Ok(None) => unreachable!("Accept has returned 'None'"),
                        Ok(Some((sock, _addr))) => sock
                    };

                    let new_token = Token(self.token_counter);
                    self.clients.insert(new_token, WebSocketClient::new(client_socket));

                    event_loop.register(&self.clients[&new_token].socket,
                                        new_token, EventSet::readable(),
                                        PollOpt::edge() | PollOpt::oneshot()).unwrap();

                    println!("Added a new client, with token counter: {}", self.token_counter);

                    self.token_counter += 1;

                },
                token => {
                    let client = self.clients.get_mut(&token).unwrap();
                    client.read();
                    println!("Read from client: {}", token.as_usize());
                    event_loop.reregister(&client.socket, token, client.interest,
                                        PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
            }
        }

        if events.is_writable() {
            let client = self.clients.get_mut(&token).unwrap();
            client.write();
            println!("Write from client: {}", token.as_usize());
            event_loop.reregister(&client.socket, token, client.interest,
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
        }

        if events.is_hup() {
            let client = self.clients.remove(&token).unwrap();

            client.socket.shutdown(Shutdown::Both).unwrap();
            event_loop.deregister(&client.socket).unwrap();
        }
    }
}
