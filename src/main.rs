extern crate http_muncher;
extern crate mio;

use std::collections::HashMap;
use std::net::SocketAddr;

use http_muncher::{Parser, ParserHandler};

use mio::*;
use mio::tcp::*;

struct HttpParser;
impl ParserHandler for HttpParser { }

struct WebSocketClient {
    socket: TcpStream,
    http_parser: Parser<HttpParser>
}

impl WebSocketClient {
    fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf) {
                Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    return
                },
                Ok(None) =>
                    // Socket buffer has got no more bytes.
                    break,
                Ok(Some(len)) => {
                    self.http_parser.parse(&buf[0..len]);
                    if self.http_parser.is_upgrade() {
                        // ...
                        break;
                    }
                }
            }
        }
    }

    fn new(socket: TcpStream) -> WebSocketClient {
        WebSocketClient {
            socket: socket,
            http_parser: Parser::request(HttpParser)
        }
    }
}

struct WebSocketServer {
    socket: TcpListener,
    clients: HashMap<Token, WebSocketClient>,
    token_counter: usize
}

const SERVER_TOKEN: Token = Token(0);

impl Handler for WebSocketServer {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<WebSocketServer>,
             token: Token,
             _events: EventSet) {
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
                self.token_counter += 1;

                event_loop.register(&self.clients[&new_token].socket,
                                    new_token, EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot()).unwrap();
            },
            token => {
                let client = self.clients.get_mut(&token).unwrap();
                client.read();
                event_loop.reregister(&client.socket, token, EventSet::readable(),
                                      PollOpt::edge() | PollOpt::oneshot()).unwrap();
            }
        }
    }
}

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut server = WebSocketServer {
        token_counter: 1,
        clients: HashMap::new(),
        socket: server_socket
    };

    event_loop.register(&server.socket,
                        Token(0),
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    event_loop.run(&mut server).unwrap();
}
