use ::os_input::{OsInput, TextChar};

use winit::VirtualKeyCode;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct CommandLine {
    history_index: isize,
    history:       Vec<String>,
    current:       Option<String>,
    output:        Vec<String>,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            history_index: -1,
            history:       vec!(),
            current:       None,
            output:        vec!(), // TODO: Really going to need to truncate/filter what goes in here or the copies will get laggy on say "package:get"
        }
    }

    pub fn step<T>(&mut self, os_input: &OsInput, root_node: &mut T) where T: Node {
        match self.current.clone() {
            Some(mut current) => {
                for text_char in os_input.text() {
                    match text_char {
                        TextChar::Char(c) => { current.push(c); }
                        TextChar::Back    => { current.pop(); }
                    }
                    self.history_index = -1;
                }
                self.current = Some(current.clone());

                if os_input.key_pressed(VirtualKeyCode::Escape) {
                    self.current = None;
                    self.history_index = -1;
                }
                if os_input.key_pressed(VirtualKeyCode::Return) {
                    self.output.insert(0, current.clone());
                    self.output.insert(
                        0,
                        match NodeRunner::new(current.as_str()) {
                            Ok(runner) => root_node.node_step(runner),
                            Err(msg)   => msg
                        }
                    ); // TODO: Split by newlines/every 1000 chars into seperate String

                    self.history.insert(0, current);
                    self.history_index = -1;
                    self.current = None;
                }
                if os_input.key_pressed(VirtualKeyCode::Up) && self.history_index + 1 < self.history.len() as isize {
                    self.history_index += 1;
                    self.current = Some(self.history[self.history_index as usize].clone());
                }
                if os_input.key_pressed(VirtualKeyCode::Down) {
                    if self.history_index > 0 {
                        self.history_index -= 1;
                        self.current = Some(self.history[self.history_index as usize].clone());
                    }
                    else if self.history_index == 0 {
                        self.history_index -= 1;
                        self.current = Some(String::new());
                    }
                }
            }
            None => {
                if os_input.key_pressed(VirtualKeyCode::Semicolon) {
                    self.current = Some(String::new());
                }
            }
        }
    }

    /// The command line is currently in use.
    /// Therefore no other inputs should be accepted.
    pub fn block(&self) -> bool {
        self.current.is_some()
    }

    /// Get the text currently entered in the command line
    pub fn output(&self) -> Vec<String> {
        match self.current {
            Some(ref current) => {
                let mut output = self.output.clone();
                output.insert(0, current.clone());
                output
            }
            None => { vec!() }
        }
    }
}
