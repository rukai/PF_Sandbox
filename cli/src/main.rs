use std::env;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::str;
use std::thread;
use std::time::Duration;

fn main() {
    let mut args = env::args();
    args.next();
    let out_vec: Vec<String> = args.collect();
    let out: String = format!("C{}", out_vec.join(" "));

    let mut stream = match TcpStream::connect("127.0.0.1:1613") {
        Ok(stream)  => { stream }
        Err(e)      => { println!("Could not connect to PF ENGINE host: {}", e); return; }
    };

    stream.write(out.as_bytes()).unwrap();

    let mut result = String::new();
    match stream.read_to_string(&mut result) {
        Ok(amt) => {
            println!("{}", result);
        },
        _ => { return; },
    }
}
