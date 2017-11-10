use treeflection::{NodeRunner, Node};
use std::net::{TcpListener, UdpSocket, IpAddr, SocketAddr};
use std::io::Read;
use std::io::Write;
use std::str;
use std::time::{Instant, Duration};

use ::input::ControllerInput;

pub struct NetCommandLine {
    listener: TcpListener
}

impl NetCommandLine {
    pub fn new() -> NetCommandLine {
        let listener = TcpListener::bind("127.0.0.1:1613").unwrap();
        listener.set_nonblocking(true).unwrap();

        NetCommandLine {
            listener: listener,
        }
    }

    pub fn step<T>(&mut self, root_node: &mut T) where T: Node {
        let mut buf = [0; 1024];
        if let Ok((mut stream, _)) = self.listener.accept() {
            match stream.read(&mut buf) {
                Ok(amt) => {
                    if amt > 1 {
                        if let Ok(string) = str::from_utf8(&buf[1..amt]) {
                            if buf[0] == 0x43 { // 'C'
                                let out = NetCommandLine::run_inner(&string, root_node);
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

    fn run_inner<T>(command: &str, package: &mut T) -> String where T: Node {
        match NodeRunner::new(command) {
            Ok(runner) => {
                let result = package.node_step(runner);
                result
            },
            Err(msg) => msg
        }
    }
}

pub struct Netplay {
    socket:             UdpSocket,
    state:              NetplayState,
    hash_msgs:          Vec<String>,
    ping_msgs:          Vec<u8>,
    css_msgs:           Vec<usize>,
    start_request_msgs: Vec<usize>,
    start_confirm_msgs: Vec<usize>,
    in_game_msgs:       Vec<usize>,
}

/* Message Codes:
 * 1   - package hash
 * 2   - ping request
 * 3   - ping response
 * 100 - quitting
*/
impl Netplay {
    pub fn new() -> Netplay {
        let socket = UdpSocket::bind("0.0.0.0:8413").unwrap();
        socket.set_nonblocking(true).unwrap();
        Netplay {
            socket,
            state:              NetplayState::Offline,
            hash_msgs:          vec!(),
            ping_msgs:          vec!(),
            css_msgs:           vec!(),
            start_request_msgs: vec!(),
            start_confirm_msgs: vec!(),
            in_game_msgs:       vec!(),
        }
    }

    /// Call this once every frame
    pub fn step(&mut self) {
        // receive messages
        {
            let mut buf = [0; 1024];
            if let Ok(buf_max) = self.socket.recv(&mut buf) { // returns Err if there is no packet waiting
                match buf[0] {
                    1 => {
                        if let Ok(hash) = String::from_utf8(buf[1..buf_max].iter().cloned().collect()) {
                            self.hash_msgs.push(hash);
                        }
                    }
                    2 => {
                        self.socket.send(&[3, buf[1]]).unwrap();
                    }
                    3 => {
                        self.ping_msgs.push(buf[1]);
                    }
                    100 => {
                        self.state = NetplayState::Disconnected { reason: String::from("Peer disconnected") };
                    }
                    _ => {
                        println!("Couldn't process netplay message starting with: {:?}", &buf[0..32]);
                    }
                }
            }
        }

        // process messages
        let mut new_state: Option<NetplayState> = None;
        match &mut self.state {
            &mut NetplayState::Disconnected { .. } => { }
            &mut NetplayState::Offline => { }
            &mut NetplayState::ComparePackageHash { ref hash } => {
                // send hash
                let mut buf = [1; 65];
                for (i, b) in hash.as_bytes().iter().enumerate() {
                    buf[i+1] = *b;
                }
                self.socket.send(&buf).unwrap();

                // receive hash
                for hash_msg in self.hash_msgs.iter() {
                    if hash_msg == hash {
                        new_state = Some(NetplayState::PingTest { pings: [Ping::default(); 255] });
                    }
                    else {
                        new_state = Some(NetplayState::Disconnected { reason: String::from("Package hashes did not match, ensure you are both using the same package.") });
                    }
                }
            }
            &mut NetplayState::PingTest { ref mut pings } => {
                // request a ping from peer and record the time_sent
                if let Some(next_ping) = pings.iter().enumerate().find(|x| x.1.time_sent.is_none()).map(|x| x.0) {
                    self.socket.send(&[2, next_ping as u8]).unwrap();
                    pings[next_ping].time_sent = Some(Instant::now());

                    // record the time_received of received pings
                    for ping_msg in self.ping_msgs.iter() {
                        pings[*ping_msg as usize].time_received = Some(Instant::now());
                    }
                }
                else {
                    let mut ping_total = Duration::from_secs(0);
                    for ping in pings.iter().take(225) { // skip the last 30 as we dont want the most recent packets showing up as dropped.
                        if let (Some(time_sent), Some(time_received)) = (ping.time_sent, ping.time_received) {
                            ping_total += time_received.duration_since(time_sent);
                        } else {
                            ping_total += Duration::from_millis(200); // punish for dropping packet
                        }
                    }

                    let ping_total = ping_total.as_secs() as f64 + ping_total.subsec_nanos() as f64 / 1_000_000_000.0;
                    let ping_avg = ping_total / 255.0;
                    println!("netplay ping: {}", ping_avg);
                    let ping_max = 100.0; // TODO: Grab from config
                    if ping_avg > ping_max {
                        // TODO: send disconnect notification to peer
                        self.socket.send(&[100]).unwrap();
                        new_state = Some(NetplayState::Disconnected { reason: format!("The ping was '{}' which was above the limit of '{}'", ping_avg, ping_max) });
                    } else {
                        new_state = Some(NetplayState::CSS { players: vec!() });
                    }
                }
                self.ping_msgs.clear();
            }
            &mut NetplayState::CSS { ref mut players } => {
                players.clear();
                // TODO: read from socket into players
            }
            &mut NetplayState::StartRequested { .. } => { }
            &mut NetplayState::StartConfirmed { .. } => { }
            &mut NetplayState::InGame { ref mut inputs } => {
                inputs.clear();
                // TODO: read from socket into inputs
            }
            &mut NetplayState::Results => { }
        }
        if let Some(state) = new_state {
            self.state = state;
        }
    }

    pub fn state(&self) -> NetplayState {
        self.state.clone()
    }

    pub fn connect(&mut self, address: IpAddr, hash: String) {
        self.socket.connect(SocketAddr::new(address, 8413)).unwrap();
        self.state = NetplayState::ComparePackageHash { hash };

        // clear messages
        self.hash_msgs.clear();
        self.ping_msgs.clear();
        self.css_msgs.clear();
        self.start_request_msgs.clear();
        self.start_confirm_msgs.clear();
        self.in_game_msgs.clear();
    }

    pub fn disconnect(&mut self) {
        match &self.state {
            &NetplayState::Offline |
            &NetplayState::Disconnected { .. } => { }
            _ => {
                self.socket.send(&[100]).unwrap();
                self.state = NetplayState::Disconnected { reason: String::from("Disconnect requested by self") };
            }
        }
    }

    pub fn disconnect_offline(&mut self) {
        match &self.state {
            &NetplayState::Offline => { }
            &NetplayState::Disconnected { .. } => { }
            _ => {
                self.socket.send(&[100]).unwrap();
                self.state = NetplayState::Offline;
            }
        }
    }

    pub fn offline(&mut self) {
        self.state = NetplayState::Offline;
    }

    pub fn start_request(&mut self, players: Vec<NetplayPlayerSelect>) {
        // TODO: self.socket.send_data();
        self.state = NetplayState::StartRequested { players };
    }

    pub fn send_css_state(&mut self, _css_states: &[NetplayPlayerSelect]) {
        // TODO: self.socket.send_data();
    }

    pub fn send_controller_inputs(&mut self, _input_confirms: &[NetplayInputConfirm]) {
        // TODO: self.socket.send_data();
    }
}

/// Possible state flow sequences:
/// *   Offline -> ComparePackageHash -> Disconnected -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> Disconnected -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Disconnect -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Start Requested -> Disconnect -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> Disconnected -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Disconnected -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Results -> Disconnected -> Offline
/// *   Offline -> ComparePackageHash -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Results -> CSS -> ... -> Disconnected -> Offline
/// Disconnected can occur due to self request, peer request, or timeout.
#[derive(Clone)]
pub enum NetplayState {
    Offline,
    Disconnected       { reason:  String },
    ComparePackageHash { hash:    String },
    PingTest           { pings:   [Ping; 255] },
    CSS                { players: Vec<NetplayPlayerSelect> },
    StartRequested     { players: Vec<NetplayPlayerSelect> },
    StartConfirmed     { players: Vec<NetplayPlayerSelect> },
    InGame             { inputs:  Vec<NetplayInputConfirm> },
    Results
}

#[derive(Clone, Default, Copy)]
pub struct Ping {
    time_sent:     Option<Instant>,
    time_received: Option<Instant>,
}

#[derive(Clone)]
pub struct NetplayInputConfirm {
    controller: ControllerInput,
    player:     usize,
    frame:      usize,
}

#[derive(Clone)]
pub struct NetplayPlayerSelect {
    player: usize,
    figher: Option<usize>,
    team:   usize,
}
