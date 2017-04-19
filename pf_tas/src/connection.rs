use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

use serde_json;

use state::State;

pub fn send(state: &mut State) {
    let mut stream = match TcpStream::connect("127.0.0.1:1613") {
        Ok(stream)  => { stream }
        Err(e)      => { println!("Could not connect to PF Sandbox host: {}", e); return }
    };
    
    let controllers: Vec<_> = state.controllers.iter_mut().map(|x| x.to_sandbox()).collect();
    let json = serde_json::to_string(&controllers).unwrap();
    let out = format!("Ctas set {}", json);
    stream.write(out.as_bytes()).unwrap();

    let mut result = String::new();
    if let Ok(_) = stream.read_to_string(&mut result) {
        println!("{}", result);
    }
}
