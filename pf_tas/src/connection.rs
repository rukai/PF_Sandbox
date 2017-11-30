use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

use serde_json;

use state::State;

pub fn send(state: &mut State) {
    if state.new_game_state.should_send() {
        let controllers: Vec<_> = state.controllers.iter_mut().map(|x| x.to_sandbox()).collect();
        let json = serde_json::to_string(&controllers).unwrap();
        send_string(format!(r#"Ctas:set "{}""#, escape(json)));

        let json = serde_json::to_string(&state.new_game_state).unwrap();
        send_string(format!(r#"Cstate:set "{}""#, escape(json)));
    }
}

pub fn quit() {
    send_string(format!(r#"Ctas:reset"#));
}

fn send_string(string: String) {
    println!("sent: {}", string);
    let mut stream = match TcpStream::connect("127.0.0.1:1613") {
        Ok(stream)  => { stream }
        Err(e)      => { println!("Could not connect to PF Sandbox host: {}", e); return }
    };

    stream.write(string.as_bytes()).unwrap();

    let mut result = String::new();
    if let Ok(_) = stream.read_to_string(&mut result) {
        println!("received: {}", result);
    }
}

fn escape(input: String) -> String {
    let mut output = String::new();
    for c in input.chars() {
        if c == '"' {
            output.push('\\');
        }
        output.push(c);
    }
    output
}
