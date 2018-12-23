mod server { pub mod server; }
mod client { pub mod client; }
mod frame  { pub mod frame;  }

use std::net::SocketAddr;

use mio::*;
use mio::tcp::*;

use crate::server::server::WebSocketServer;

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut server = WebSocketServer::new(server_socket);

    event_loop.register(&server.socket,
                        Token(0),
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    event_loop.run(&mut server).unwrap();
}
