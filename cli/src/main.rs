use std::net::UdpSocket;
use std::str;
use std::env;

fn main() {
    let mut args = env::args();
    args.next();
    let out_vec: Vec<String> = args.collect();
    let out: String = format!("C{}", out_vec.join(" "));

    // Using "localhost:1614" will send via ipv6 ... wat o.0
    let socket = match UdpSocket::bind("127.0.0.1:1614") {
        Ok(socket)  => { socket }
        Err(_)      => { println!("Port 1614 is not available"); return; }
    };

    socket.connect("127.0.0.1:1613").unwrap();

    socket.send(out.as_bytes()).unwrap();

    let mut buf = [0; 1000];
    match socket.recv(&mut buf) {
        Ok(amt) => {
            if let Ok(string) = str::from_utf8(&buf[0..amt]) {
                println!("{}", string);
            }
        },
        _ => { return; },
    }
}
