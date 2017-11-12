use treeflection::{NodeRunner, Node};
use bincode;
use byteorder::{ByteOrder, NetworkEndian};
use rand::Rng;
use rand;
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

/*  Message Formats:

    Initiate Connection:
        1 byte   - 0x01
        64 bytes - package hash
        8 bytes  - random number to determine client order

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
    pub confirmed_inputs: Vec<Vec<Vec<ControllerInput>>>,
    seed:                 u64,
    socket:               UdpSocket,
    state:                NetplayState,
    frame_confirmed:      usize, // TODO: Move to enum?!?!
    prev_frame_confirmed: usize,
    index:                usize,
    init_msgs:            Vec<(String, u64)>,
    ping_msgs:            Vec<u8>,
    css_msgs:             Vec<usize>,
    start_request_msgs:   Vec<usize>,
    start_confirm_msgs:   Vec<usize>,
    running_msgs:         Vec<InputConfirm>,
}

impl Netplay {
    pub fn new() -> Netplay {
        let socket = UdpSocket::bind("0.0.0.0:8413").unwrap();
        socket.set_nonblocking(true).unwrap();
        Netplay {
            socket,
            state:                NetplayState::Offline,
            confirmed_inputs:     vec!(),
            frame_confirmed:      0,
            prev_frame_confirmed: 0,
            seed:                 0,
            index:                0,
            init_msgs:            vec!(),
            ping_msgs:            vec!(),
            css_msgs:             vec!(),
            start_request_msgs:   vec!(),
            start_confirm_msgs:   vec!(),
            running_msgs:         vec!(),
        }
    }

    /// Call this once every frame
    pub fn step(&mut self) {
        // receive messages
        loop {
            let mut buf = [0; 1024];
            if let Ok(_) = self.socket.recv(&mut buf) { // returns Err if there is no packet waiting
                match buf[0] {
                    0x01 => {
                        if let Ok(hash) = String::from_utf8(buf[1..65].iter().cloned().collect()) {
                            let random_bytes = &buf[65..73];
                            let random = NetworkEndian::read_u64(random_bytes);
                            self.init_msgs.push((hash, random));
                        }
                    }
                    0x02 => {
                        self.socket.send(&[3, buf[1]]).unwrap();
                    }
                    0x03 => {
                        self.ping_msgs.push(buf[1]);
                    }
                    0x04 => {
                        let mut data = bincode::deserialize(&buf[1..]).unwrap();
                        self.running_msgs.push(data);
                    }
                    0xAA => {
                        self.state = NetplayState::Disconnected { reason: String::from("Peer disconnected") };
                    }
                    _ => {
                        println!("Couldn't process netplay message starting with: {:?}", &buf[0..32]);
                    }
                }
            }
            else {
                break;
            }
        }

        // process messages
        let mut new_state: Option<NetplayState> = None;
        match &mut self.state {
            &mut NetplayState::Disconnected { .. } => { }
            &mut NetplayState::Offline => { }
            &mut NetplayState::InitConnection { ref hash, random } => {
                // send hash
                let mut buf = [1; 73];
                for (i, b) in hash.as_bytes().iter().enumerate() {
                    buf[i+1] = *b;
                }
                NetworkEndian::write_u64(&mut buf[65..73], random);
                self.socket.send(&buf).unwrap();

                // receive hash
                for (hash_msg, random_msg) in self.init_msgs.drain(..) {
                    if &hash_msg == hash {
                        new_state = Some(NetplayState::PingTest { pings: [Ping::default(); 255] });
                    }
                    else {
                        new_state = Some(NetplayState::Disconnected { reason: String::from("Package hashes did not match, ensure you are both using the same package.") });
                    }

                    // TODO: handle multiple peers
                    if random < random_msg {
                        self.index = 0;
                        self.seed = random;
                    }
                    else {
                        self.index = 1;
                        self.seed = random_msg;
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
                    let ping_max = 100.0; // TODO: Grab from config
                    if ping_avg > ping_max {
                        self.socket.send(&[0xAA]).unwrap();
                        new_state = Some(NetplayState::Disconnected { reason: format!("The ping was '{}' which was above the limit of '{}'", ping_avg, ping_max) });
                    } else {
                        new_state = Some(NetplayState::Running { frame: 0 });
                        // TODO: Need to force input reset all history at this point
                    }
                }
                self.ping_msgs.clear();
            }
            &mut NetplayState::Running { .. } => {
                for msg in self.running_msgs.drain(..) {
                    let peer = 0; // TODO: handle multiple peers
                    self.confirmed_inputs[peer].push(msg.inputs);
                }
                self.running_msgs.clear();
            }
        }
        if let Some(state) = new_state {
            self.state = state;
        }
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
        match &self.state {
            &NetplayState::Running { .. } => 2, // TODO: handle multiple peers
            _ => 1
        }
    }

    pub fn connect(&mut self, address: IpAddr, hash: String) {
        if let Err(err) = self.socket.connect(SocketAddr::new(address, 8413)) {
            self.state = NetplayState::Disconnected { reason: format!("Can't connect to network: {}", err) };
        }
        else {
            let random = rand::thread_rng().gen::<u64>();
            self.state = NetplayState::InitConnection { hash, random };
            self.confirmed_inputs.clear();
            self.confirmed_inputs.push(vec!()); // TODO: handle multiple peers

            // clear messages
            self.init_msgs.clear();
            self.ping_msgs.clear();
            self.css_msgs.clear();
            self.start_request_msgs.clear();
            self.start_confirm_msgs.clear();
            self.running_msgs.clear();
        }
    }

    pub fn disconnect(&mut self) {
        match &self.state {
            &NetplayState::Offline |
            &NetplayState::Disconnected { .. } => { }
            _ => {
                self.socket.send(&[0xAA]).ok();
                self.state = NetplayState::Disconnected { reason: String::from("Disconnect requested by self") };
            }
        }
    }

    pub fn disconnect_offline(&mut self) {
        match &self.state {
            &NetplayState::Offline => { }
            &NetplayState::Disconnected { .. } => { }
            _ => {
                self.socket.send(&[0xAA]).ok();
                self.state = NetplayState::Offline;
            }
        }
    }

    pub fn offline(&mut self) {
        self.state = NetplayState::Offline;
    }

    pub fn send_controller_inputs(&mut self, inputs: Vec<ControllerInput>) {
        if let &NetplayState::Running { frame, .. } = &self.state {
            let input_confirm = InputConfirm {
                frame,
                inputs
            };
            let mut data = bincode::serialize(&input_confirm, bincode::Infinite).unwrap();
            data.insert(0, 0x04);
            self.socket.send(&data).unwrap();
        }
        // TODO: Store InputConfirm so we can resend it later (maybe repeat it every step() for n steps, no idea how to best handle this sort of thing)
    }

    // Use peer 0's random value to generate the game seed for all games in the current session.
    // Repeating seeds like this shouldnt be noticeable
    pub fn get_seed(&self) -> Option<u64> {
        match &self.state {
            &NetplayState::Running { .. } => {
                Some(self.seed)
            }
            _ => None
        }
    }
}

/// Possible state flow sequences:
/// *   Offline -> InitConnection -> Disconnected -> Offline
/// *   Offline -> InitConnection -> Ping Test -> Disconnected -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Disconnect -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Start Requested -> Disconnect -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> Disconnected -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Disconnected -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Results -> Disconnected -> Offline
/// *   Offline -> InitConnection -> Ping Test -> CSS -> Start Requested -> StartConfirmed -> InGame -> Results -> CSS -> ... -> Disconnected -> Offline
/// Disconnected can occur due to self request, peer request, or timeout.
#[derive(Clone)]
pub enum NetplayState {
    Offline,
    Disconnected       { reason: String },
    InitConnection     { hash:   String, random: u64 },
    PingTest           { pings:  [Ping; 255] },
    Running            { frame:  usize },
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
