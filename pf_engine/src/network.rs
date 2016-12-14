use treeflection::{NodeRunner, Node};

use std::net::UdpSocket;
use std::str;

/// Network is used to handle both:
/// *   remote commands
/// *   netplay
pub struct Network {
    socket: UdpSocket,
}

impl Network {
    pub fn new() -> Network {
        let socket = UdpSocket::bind("127.0.0.1:1613").unwrap();
        socket.set_nonblocking(true).unwrap(); // TODO: Err ... why can this even fail?

        Network {
            socket: socket,
        }
    }

    pub fn update<T>(&mut self, root_node: &mut T) where T: Node {
        loop {
            let mut buf = [0; 1000]; // TODO: Err ... how big should this thing be?
            match self.socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if let Ok(string) = str::from_utf8(&buf[1..amt]) {
                        if buf[0] == 0x43 { // 'C'
                            let out = Network::run_command(&string, root_node);
                            self.socket.send_to(out.as_bytes(), &src).unwrap();
                        }
                    }
                },
                _ => { return; },
            }
        }
    }

    pub fn run_command<T>(command: &str, package: &mut T) -> String where T: Node{
        match NodeRunner::new(command) {
            Ok(runner) => {
                let result = package.node_step(runner);
                result
            },
            Err(msg) => msg
        }
    }
}
