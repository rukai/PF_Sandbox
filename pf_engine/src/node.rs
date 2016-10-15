pub trait Node {
    fn node_step(&mut self, runner: NodeRunner) -> String;
}

impl<U> Node for Vec<U> where U: Node {
    fn node_step(&mut self, mut runner: NodeRunner) -> String {
        match runner.step() {
            NodeToken::ChainIndex (index) => {
                let length = self.len();
                match self.get_mut(index) {
                    Some (item) => item.node_step(runner),
                    None      => return format!("Used index {} on a list of size {} (try a value between 0-{}", index, length, length)
                }
            },
            NodeToken::ChainProperty (ref s) if s == "length" => { self.len().node_step(runner) }
            action => { format!("List cannot '{:?}'", action) }
        }
    }
}

impl Node for usize {
    fn node_step(&mut self, mut runner: NodeRunner) -> String {
        match runner.step() {
            NodeToken::Get         => { return (*self).to_string() }
            NodeToken::Set (value) => { *self = value.parse().unwrap() }
            action                  => { return format!("usize cannot '{:?}'", action) }
        };
        String::from("")
    }
}

pub struct NodeRunner {
    tokens: Vec<NodeToken>
}

impl NodeRunner {
    pub fn new(command: &str) -> Result<NodeRunner, String> {
        // add first identifier to token as property
        // get next identifier, could be:
        // *   ChainProperty - starts with '.'
        // *   ChainKey      - starts with '[0-9' ends with ']'
        // *   ChainIndex    - starts with '[a-z' ends with ']'
        // repeat until space found
        // then add identifier as action including any arguments seperated by spaces
        let mut tokens: Vec<NodeToken> = vec!();
        let mut token_progress = NodeTokenProgress::ChainProperty;
        let mut token_begin = 0;

        let chars: Vec<char> = command.chars().collect();
        for (i, c_ref) in chars.iter().enumerate() {
            let c = *c_ref;
            if c == '.' || c == ' ' || c == '[' {
                tokens.push(match token_progress {
                    NodeTokenProgress::ChainProperty => {
                        let token_str = &command[token_begin..i];
                        if token_str.len() == 0 {
                            return Err (String::from("Missing property"));
                        }
                        NodeToken::ChainProperty (token_str.to_string())
                    }

                    NodeTokenProgress::ChainIndex => {
                        let token_str = &command[token_begin..i-1];
                        if token_str.len() == 0 {
                            return Err (String::from("Missing index"));
                        }
                        match command[token_begin..i-1].parse() {
                            Ok (index) => NodeToken::ChainIndex (index),
                            Err (_)    => return Err (String::from("Not a valid index"))
                        }
                    }

                    NodeTokenProgress::ChainKey => {
                        let token_str = &command[token_begin..i-1];
                        if token_str.len() == 0 {
                            return Err (String::from("Missing index"));
                        }
                        NodeToken::ChainKey (token_str.to_string())
                    }
                    NodeTokenProgress::Action => {
                        NodeToken::Get
                    }
                });
                token_begin = i+1;
            }

            match c {
                '.' => {
                    token_progress = NodeTokenProgress::ChainProperty;
                }
                ' ' => {
                    token_progress = NodeTokenProgress::Action;
                    break;
                }
                '[' => {
                    if let Some(next_c) = chars.get(i+1) {
                        if next_c.is_digit(10) {
                            token_progress = NodeTokenProgress::ChainIndex;
                        }
                        else if next_c.is_alphabetic() {
                            token_progress = NodeTokenProgress::ChainKey;
                        }
                        else {
                            return Err (String::from("Not a valid key or index."));
                        }
                    }
                    else {
                        return Err (String::from("Unfinished key or index."));
                    }
                }
                _ => { }
            }
        }

        // add action
        if let NodeTokenProgress::Action = token_progress {
            let mut action = command[token_begin..].split_whitespace();
            tokens.push(match action.next() {
                Some("get") => NodeToken::Get,
                Some("set") => {
                    match action.next() {
                        Some(arg) => NodeToken::Set(arg.to_string()),
                        None => return Err (String::from("No argument given to set action"))
                    }
                }
                Some("copy")  => NodeToken::CopyFrom,
                Some("paste") => NodeToken::PasteTo,
                Some(&_)      => return Err (String::from("Action is invalid")), // TODO: Custom actions
                None          => return Err (String::from("This should be unreachable: No Action"))
            });
        }
        else {
            return Err (String::from("No action"));
        }

        tokens.reverse();
        println!("{:?}", tokens);

        Ok(NodeRunner {
            tokens: tokens
        })
    }

    pub fn step(&mut self) -> NodeToken {
        self.tokens.pop().unwrap()
    }
}

#[derive(Debug)]
pub enum NodeTokenProgress {
    ChainProperty,
    ChainIndex,
    ChainKey,
    Action
}

#[derive(Debug)]
pub enum NodeToken {
    ChainProperty (String),
    ChainIndex (usize),
    ChainKey (String),
    Get,
    Set (String),
    CopyFrom,
    PasteTo,
    Custom (String),
}
