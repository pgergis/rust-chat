extern crate http_muncher;
extern crate mio;
extern crate rustc_serialize;
extern crate sha1;

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::rc::Rc;

use http_muncher::{Parser, ParserHandler};

use mio::*;
use mio::tcp::*;

use rustc_serialize::base64::{ToBase64, STANDARD};

fn gen_key(key: &String) -> String {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    m.update(key.as_bytes());
    m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

    m.output(&mut buf);

    return buf.to_base64(STANDARD);
}

struct HttpParser {
    current_key: Option<String>,
    headers: Rc<RefCell<HashMap<String, String>>>
}

impl ParserHandler for HttpParser {
    fn on_header_field(&mut self, s: &[u8]) -> bool {
        self.current_key = Some(std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, s: &[u8]) -> bool {
        self.headers.borrow_mut()
            .insert(self.current_key.clone().unwrap(),
                    std::str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_headers_complete(&mut self) -> bool {
        false
    }
}

enum ClientState {
    AwaitingHandshake(RefCell<Parser<HttpParser>>),
    HandshakeResponse,
    Connected
}

struct WebSocketClient {
    socket: TcpStream,
    headers: Rc<RefCell<HashMap<String, String>>>,
    interest: EventSet,
    state: ClientState
}

impl WebSocketClient {
    fn new(socket: TcpStream) -> WebSocketClient {
        let headers = Rc::new(RefCell::new(HashMap::new()));

        WebSocketClient {
            socket: socket,
            // need an initial clone of 'headers' to read its contents
            // (it's being shared with HttpParser)
            headers: headers.clone(),
            interest: EventSet::readable(),
            state: ClientState::AwaitingHandshake(RefCell::new(Parser::request(HttpParser {
                current_key: None,
                // the second clone to let the parser write to headers
                headers: headers.clone()
            })))
        }
    }

    fn read(&mut self) {
        match self.state {
            ClientState::AwaitingHandshake(_) => {
                self.read_handshake();
            },
            _ => {}
        }
    }

    fn read_handshake(&mut self) {
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
                Ok(Some(_len)) => {
                    let is_upgrade = if let ClientState::AwaitingHandshake(ref parser_state) = self.state {
                        let mut parser = parser_state.borrow_mut();
                        // parser.parse(&buf[0..len]);
                        parser.parse(&buf);
                        parser.is_upgrade()
                    } else { false };

                    if is_upgrade {
                        self.state = ClientState::HandshakeResponse;
                        self.interest.remove(EventSet::readable());
                        self.interest.insert(EventSet::writable());

                        break;
                    }
                }
            }
        }
    }

    fn write(&mut self) {
        let headers = self.headers.borrow();
        let response_key = gen_key(&headers.get("Sec-WebSocket-Key").unwrap());

        let response = std::fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                                 Connection: Upgrade\r\n\
                                                 Sec-WebSocket-Accept: {}\r\n\
                                                 Upgrade: websocket\r\n\r\n", response_key));

        self.socket.try_write(response.as_bytes()).unwrap();
        self.state = ClientState::Connected;

        self.interest.remove(EventSet::writable());
        self.interest.insert(EventSet::readable());
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
                    self.token_counter += 1;

                    event_loop.register(&self.clients[&new_token].socket,
                                        new_token, EventSet::readable(),
                                        PollOpt::edge() | PollOpt::oneshot()).unwrap();
                },
                token => {
                    let client = self.clients.get_mut(&token).unwrap();
                    client.read();
                    event_loop.reregister(&client.socket, token, client.interest,
                                        PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
            }
        }

        if events.is_writable() {
            let client = self.clients.get_mut(&token).unwrap();
            client.write();
            event_loop.reregister(&client.socket, token, client.interest,
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
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
