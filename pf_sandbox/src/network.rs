use treeflection::{NodeRunner, Node};
use std::net::TcpListener;
use std::io::Read;
use std::io::Write;
use std::str;

/// Network is used to handle both:
/// *   remote commands
/// *   netplay
pub struct Network {
    listener: TcpListener
}

impl Network {
    pub fn new() -> Network {
        let listener = TcpListener::bind("127.0.0.1:1613").unwrap();
        listener.set_nonblocking(true).unwrap();

        Network {
            listener: listener,
        }
    }

    pub fn update<T>(&mut self, root_node: &mut T) where T: Node {
        let mut buf = [0; 1024];
        if let Ok((mut stream, _)) = self.listener.accept() {
            match stream.read(&mut buf) {
                Ok(amt) => {
                    if amt > 1 {
                        if let Ok(string) = str::from_utf8(&buf[1..amt]) {
                            if buf[0] == 0x43 { // 'C'
                                let out = Network::run_command(&string, root_node);
                                if let Err(e) = stream.write(out.as_bytes()) {
                                    println!("command send failed {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("command receive failed {}", e);
                }
            }
        }
    }

    pub fn run_command<T>(command: &str, package: &mut T) -> String where T: Node {
        match NodeRunner::new(command) {
            Ok(runner) => {
                let result = package.node_step(runner);
                result
            },
            Err(msg) => msg
        }
    }
}
