use treeflection::{NodeRunner, Node};
use bincode;
use rand::Rng;
use rand;
use json_upgrade;

use std::net::{TcpListener, UdpSocket, IpAddr, SocketAddr};
use std::io::Read;
use std::io::Write;
use std::str;
use std::time::{Instant, Duration};

use input::ControllerInput;

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

/*  Message Formats:
    Matchmaking Request:
        1 byte - 0x00
        n bytes - bincode serialized MatchMakingRequest

    Initiate Connection:
        1 byte   - 0x01
        n bytes - bincode serialized InitConnection

    Ping Request:
        1 byte - 0x02
        1 byte - ping id

    Ping Response:
        1 byte - 0x03
        1 byte - ping id

    Controller Input Message
        1 byte  - 0x04
        n bytes - bincode serialized controller input data

    Disconnect notification:
        1 byte - 0xAA
*/

pub struct Netplay {
    // structure: peers Vec<frames Vec<controllers Vec<ControllerInput>>>
    // frame 0 has index 2
    pub confirmed_inputs:  Vec<Vec<Vec<ControllerInput>>>,
    match_making_response: Option<MatchMakingResponse>,
    peers:                 Vec<SocketAddr>,
    seed:                  u64,
    socket:                UdpSocket,
    state:                 NetplayState,
    state_frame:           usize,
    last_received_frame:   usize,
    index:                 usize,
    init_msgs:             Vec<InitConnection>,
    ping_msgs:             Vec<u8>,
    start_request_msgs:    Vec<usize>,
    start_confirm_msgs:    Vec<usize>,
    running_msgs:          Vec<InputConfirm>,
}

impl Netplay {
    pub fn new() -> Netplay {
        let socket = UdpSocket::bind("0.0.0.0:8413").unwrap();
        socket.set_nonblocking(true).unwrap();
        Netplay {
            state:                 NetplayState::Offline,
            state_frame:           0,
            last_received_frame:   0,
            confirmed_inputs:      vec!(),
            match_making_response: None,
            peers:                 vec!(),
            seed:                  0,
            index:                 0,
            init_msgs:             vec!(),
            ping_msgs:             vec!(),
            start_request_msgs:    vec!(),
            start_confirm_msgs:    vec!(),
            running_msgs:          vec!(),
            socket,
        }
    }

    /// Call this once every frame
    pub fn step(&mut self) {
        if !self.skip_frame() {
            self.state_frame += 1;
        }

        // receive messages
        loop {
            let mut buf = [0; 1024];
            if let Ok((_, addr)) = self.socket.recv_from(&mut buf) { // returns Err if there is no packet waiting
                match buf[0] {
                    0x00 => {
                        if let Ok(data) = bincode::deserialize(&buf[1..]) {
                            self.match_making_response = Some(data);
                        }
                    }
                    0x01 => {
                        if self.peers.contains(&addr) {
                            if let Ok(data) = bincode::deserialize(&buf[1..]) {
                                self.init_msgs.push(data);
                            }
                        }
                    }
                    0x02 => {
                        if self.peers.contains(&addr) {
                            self.socket.send_to(&[3, buf[1]], addr).unwrap();
                        }
                    }
                    0x03 => {
                        if self.peers.contains(&addr) {
                            self.ping_msgs.push(buf[1]);
                        }
                    }
                    0x04 => {
                        if self.peers.contains(&addr) {
                            if let Ok(data) = bincode::deserialize(&buf[1..]) {
                                self.running_msgs.push(data);
                            }
                        }
                    }
                    0xAA => {
                        self.disconnect_with_reason("Peer disconnected");
                    }
                    _ => {
                        println!("Couldn't process netplay message starting with: {:?}", &buf[0..32]);
                    }
                }
                self.last_received_frame = self.state_frame;
            }
            else {
                break;
            }
        }

        if self.peers.len() > 0 && self.state_frame - self.last_received_frame > 600 {
            self.disconnect_with_reason("Connection timed out: no packets received in the last 10 seconds");
        }

        // process messages
        match self.state.clone() {
            NetplayState::Offline => { }
            NetplayState::Disconnected { .. } => { }
            NetplayState::MatchMaking { request, } => {
                if self.state_frame % 600 == 1 { // Send a request every 10 seconds
                    let mut data = bincode::serialize(&request).unwrap();
                    data.insert(0, 0x00);
                    if let Err(_) = self.socket.send_to(&data, "matchmaking.pfsandbox.net:8413") {
                        self.disconnect_with_reason("matchmaking.pfsandbox.net:8413 is inaccessible");
                    }
                }
                if let &Some(ref response) = &self.match_making_response {
                    for peer in response.addresses.iter() {
                        if !self.peers.contains(peer) {
                            self.peers.push(peer.clone());
                            self.confirmed_inputs.push(vec!());
                        }
                    }
                }
                if self.peers.len() as u8 + 1 == request.num_players {
                    self.set_state(NetplayState::InitConnection (InitConnection {
                        random:        rand::thread_rng().gen::<u64>(),
                        build_version: request.build_version.clone(),
                        hash:          request.package_hash.clone()
                    }));
                }
            }
            NetplayState::InitConnection (local) => {
                // send init
                let mut data = bincode::serialize(&local).unwrap();
                data.insert(0, 0x01);
                self.broadcast(&data, "init");

                // receive init
                if let Some(init) = self.init_msgs.pop() {
                    if init.hash != local.hash {
                        self.disconnect_with_reason("Package hashes did not match, ensure everyone is using the same package.");
                    }
                    else if init.build_version != local.build_version {
                        self.disconnect_with_reason("Build versions did not match, ensure everyone is using the same PF Sandbox build.");
                    }
                    else {
                        self.set_state(NetplayState::PingTest { local_init: local.clone(), pings: [Ping::default(); 255] });
                    }

                    // Use peer 0's random value to generate the game seed for all games in the current session.
                    // Repeating seeds like this shouldnt be noticeable
                    // TODO: handle multiple peers
                    if local.random < init.random {
                        self.index = 0;
                        self.seed = local.random;
                    }
                    else {
                        self.index = 1;
                        self.seed = init.random;
                    }
                }
            }
            NetplayState::PingTest { local_init, mut pings } => {
                // if we havnt received a ping yet then resend init message
                if pings.iter().all(|x| x.time_received.is_none()) {
                    let mut data = bincode::serialize(&local_init).unwrap();
                    data.insert(0, 0x01);
                    self.broadcast(&data, "init2");
                }

                // request a ping from peer and record the time_sent
                if let Some(next_ping) = pings.iter().enumerate().find(|x| x.1.time_sent.is_none()).map(|x| x.0) {
                    self.broadcast(&[2, next_ping as u8], "ping");
                    pings[next_ping].time_sent = Some(Instant::now());

                    // record the time_received of received pings
                    for ping_msg in self.ping_msgs.iter() {
                        pings[*ping_msg as usize].time_received = Some(Instant::now());
                    }
                    self.state = NetplayState::PingTest { local_init, pings };
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
                    let ping_max = 100.0; // TODO: Grab from config
                    if ping_avg > ping_max {
                        self.disconnect_with_reason(format!("The ping was '{}' which was above the limit of '{}'", ping_avg, ping_max).as_ref());
                    } else {
                        self.set_state(NetplayState::Running);
                        // TODO: Need to force input reset all history at this point
                    }
                }
            }
            NetplayState::Running => {
                let peer = 0; // TODO: handle multiple peers
                let mut found_msg = true;
                let mut to_delete = vec!();
                while found_msg {
                    found_msg = false;
                    for (i, msg) in self.running_msgs.iter().enumerate() {
                        let inputs_len = self.confirmed_inputs[peer].len();
                        // msg.frame starts at 1 because its taken from the peers state_frame which is incremented before any logic is run
                        if msg.frame == inputs_len + 1 {
                            self.confirmed_inputs[peer].push(msg.inputs.clone());
                            found_msg = true;
                            to_delete.push(i)
                        }
                    }

                    to_delete.reverse();
                    for i in to_delete.iter() {
                        self.running_msgs.remove(*i);
                    }
                    to_delete.clear();
                }
            }
        }
        debug!("state: {}", self.state.to_string());
        debug!("number_of_peers: {}", self.number_of_peers());
        debug!("local_index: {}",  self.local_index());
        debug!("frame: {}", self.frame());
        debug!("frames_to_step: {}", self.frames_to_step());
        debug!("skip_frame: {}", self.skip_frame());
    }

    pub fn state(&self) -> NetplayState {
        self.state.clone()
    }

    /// Returns the index of the local machine
    pub fn local_index(&self) -> usize {
        match &self.state {
            &NetplayState::Running { .. } => self.index,
            _ => 0
        }
    }

    /// Returns the total number of peers including the local machine
    pub fn number_of_peers(&self) -> usize {
        self.peers.len() + 1
    }

    // TODO: Optimize by only starting from a frame where the inputs differ
    /// Returns the number of frames that need to be stepped/restepped including the current frame
    pub fn frames_to_step(&self) -> usize {
        let input_frames = self.confirmed_inputs.iter().map(|x| x.len()).min().unwrap_or(1);
        match &self.state {
            &NetplayState::Running => self.state_frame.saturating_sub(input_frames).max(1),
            _ => 1
        }
    }

    pub fn frame(&self) -> usize {
        match &self.state {
            &NetplayState::Running => self.state_frame,
            _ => 0
        }
    }

    // TODO: take ping into account
    /// Returns true if the local machine should do nothing for a frame so that peers can catch up.
    pub fn skip_frame(&self) -> bool {
        let input_frames = self.confirmed_inputs.iter().map(|x| x.len()).min().unwrap_or(1);
        match &self.state {
            &NetplayState::Running => self.state_frame > input_frames + 1,
            _ => false
        }
    }

    /// Return the seed used for this netplay session
    pub fn get_seed(&self) -> Option<u64> {
        match &self.state {
            &NetplayState::Running { .. } => {
                Some(self.seed)
            }
            _ => None
        }
    }

    fn broadcast(&mut self, message: &[u8], message_name: &str) {
        let mut fail = false;
        for peer in self.peers.iter() {
            if let Err(_) = self.socket.send_to(message, peer) {
                fail = true;
                break;
            }
        }
        if fail {
            self.disconnect_with_reason(format!("Peer is inaccessible: failed to send {}", message_name).as_ref());
        }
    }

    fn clear(&mut self) {
        self.confirmed_inputs.clear();
        self.index = 0;
        self.init_msgs.clear();
        self.last_received_frame = 0;
        self.match_making_response = None;
        self.peers.clear();
        self.ping_msgs.clear();
        self.running_msgs.clear();
        self.seed = 0;
        self.start_confirm_msgs.clear();
        self.start_request_msgs.clear();
        self.state_frame = 0;
    }

    pub fn direct_connect(&mut self, address: IpAddr, hash: String) {
        self.clear();
        self.peers.push(SocketAddr::new(address, 8413));
        self.confirmed_inputs.push(vec!());
        self.set_state(NetplayState::InitConnection (InitConnection {
            random:        rand::thread_rng().gen::<u64>(),
            build_version: json_upgrade::build_version(),
            hash
        }));
    }

    pub fn connect_match_making(&mut self, region: String, num_players: u8, package_hash: String) {
        self.clear();
        let request = MatchMakingRequest {
            build_version: json_upgrade::build_version(),
            region,
            num_players,
            package_hash,
        };
        self.set_state(NetplayState::MatchMaking { request });
    }

    fn set_state(&mut self, state: NetplayState) {
        self.state = state;
        self.state_frame = 0;
        self.last_received_frame = 0;
    }

    fn disconnect_with_reason(&mut self, reason: &str) {
        match &self.state {
            &NetplayState::Offline |
            &NetplayState::Disconnected { .. } => { }
            _ => {
                for peer in self.peers.iter() {
                    self.socket.send_to(&[0xAA], peer).ok();
                }
                self.set_state(NetplayState::Disconnected { reason: String::from(reason) });
                self.clear();
            }
        }
    }

    pub fn set_offline(&mut self) {
        match &self.state {
            &NetplayState::Offline => { }
            _ => {
                for peer in self.peers.iter() {
                    self.socket.send_to(&[0xAA], peer).ok();
                }
                self.set_state(NetplayState::Offline);
                self.clear();
            }
        }
    }

    pub fn send_controller_inputs(&mut self, inputs: Vec<ControllerInput>) {
        if let &NetplayState::Running = &self.state {
            let input_confirm = InputConfirm {
                frame: self.state_frame,
                inputs
            };
            let mut data = bincode::serialize(&input_confirm).unwrap();
            data.insert(0, 0x04);
            self.broadcast(&data, "controller input");
        }
        // TODO: Store InputConfirm so we can resend it later (maybe repeat it every step() for n steps, no idea how to best handle this sort of thing)
    }
}

/// State flow sequence:
///     Offline -> MatchMaking -> InitConnection -> Ping Test -> Running -> Disconnected -> Offline
#[derive(Clone)]
pub enum NetplayState {
    Offline,
    Running,
    InitConnection (InitConnection),
    MatchMaking    { request: MatchMakingRequest },
    Disconnected   { reason: String },
    PingTest       { local_init: InitConnection, pings:  [Ping; 255] },
}

impl NetplayState {
    pub fn to_string(&self) -> String {
        match self {
            &NetplayState::Offline               => String::from("Offline"),
            &NetplayState::Running               => String::from("Running"),
            &NetplayState::InitConnection (_)    => String::from("InitConnection"),
            &NetplayState::MatchMaking    { .. } => String::from("MatchMaking"),
            &NetplayState::Disconnected   { .. } => String::from("Disconnected"),
            &NetplayState::PingTest       { .. } => String::from("PingTest"),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct MatchMakingRequest {
    pub region:    String,
    package_hash:  String,
    build_version: String,
    num_players:   u8
}

#[derive(Clone, Deserialize)]
struct MatchMakingResponse {
    addresses: Vec<SocketAddr>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitConnection {
    build_version:  String,
    hash:           String,
    random:         u64
}

#[derive(Clone, Default, Copy)]
pub struct Ping {
    time_sent:     Option<Instant>,
    time_received: Option<Instant>,
}

#[derive(Clone, Serialize, Deserialize)]
struct InputConfirm {
    inputs: Vec<ControllerInput>,
    frame:  usize,
}
