use package::Package;
use command;

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

    pub fn update(&mut self, package: &mut Package) {
        loop {
            let mut buf = [0; 1000]; // TODO: Err ... how big should this thing be?
            match self.socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    if let Ok(string) = str::from_utf8(&buf[1..amt]) {
                        if buf[0] == 0x43 { // 'C'
                            let out = command::run(&string, package);
                            self.socket.send_to(out.as_bytes(), &src).unwrap();
                        }
                    }
                },
                _ => { return; },
            }
        }
    }
}
