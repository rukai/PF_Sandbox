use os_input::{OsInput, TextChar};

use std::collections::VecDeque;

use winit::VirtualKeyCode;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct CommandLine {
    history_index: isize,
    cursor:        usize,
    history:       Vec<String>,
    command:       String,
    output:        VecDeque<String>,
    running:       bool,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            history_index: -1,
            cursor:        0,
            history:       vec!(),
            command:       String::new(),
            output:        VecDeque::new(),
            running:       false,
        }
    }

    pub fn step<T>(&mut self, os_input: &OsInput, root_node: &mut T) where T: Node {
        if os_input.key_pressed(VirtualKeyCode::Grave) {
            self.running = !self.running;
            return;
        }

        if self.running {
            for text_char in os_input.text() {
                match text_char {
                    TextChar::Char(new_char) => {
                        let mut new_command = String::new();
                        let mut hit_cursor = false;
                        for (i, old_char) in self.command.char_indices() {
                            if i == self.cursor {
                                hit_cursor = true;
                                new_command.push(new_char);
                            }
                            new_command.push(old_char);
                        }
                        if !hit_cursor {
                            new_command.push(new_char);
                        }
                        self.command = new_command;
                        self.cursor += 1;
                    }
                    TextChar::Back => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                            let mut new_command = String::new();
                            for (i, old_char) in self.command.char_indices() {
                                if i != self.cursor {
                                    new_command.push(old_char);
                                }
                            }
                            self.command = new_command;
                        }
                    }
                }
                self.history_index = -1;
            }

            if os_input.key_pressed(VirtualKeyCode::Return) {
                {
                    let command = format!("→{}", self.command.trim_right());
                    self.output_add(command);
                }
                let result = match NodeRunner::new(self.command.as_str()) {
                    Ok(runner) => root_node.node_step(runner),
                    Err(msg)   => msg
                };
                for line in result.split('\n') {
                    self.output_add(line.to_string());
                }

                self.history.insert(0, self.command.trim_right().to_string());
                self.history_index = -1;
                self.command.clear();
                self.cursor = 0;
            }
            if os_input.key_pressed(VirtualKeyCode::Home) {
                self.cursor = 0;
            }
            if os_input.key_pressed(VirtualKeyCode::End) {
                self.cursor = self.command.chars().count();
            }
            if os_input.key_pressed(VirtualKeyCode::Left) && self.cursor > 0 {
                self.cursor -= 1;
            }
            if os_input.key_pressed(VirtualKeyCode::Right) && self.cursor < self.command.chars().count() {
                self.cursor += 1;
            }
            if os_input.key_pressed(VirtualKeyCode::Up) && self.history_index + 1 < self.history.len() as isize {
                self.history_index += 1;
                self.command = self.history[self.history_index as usize].clone();
                self.cursor = self.command.chars().count();
            }
            if os_input.key_pressed(VirtualKeyCode::Down) {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    self.command = self.history[self.history_index as usize].clone();
                    self.cursor = self.command.chars().count();
                }
                else if self.history_index == 0 {
                    self.history_index -= 1;
                    self.command.clear();
                    self.cursor = 0;
                }
            }
        }
    }

    fn output_add(&mut self, line: String) {
        if self.output.len() >= 100 {
            self.output.pop_back();
        }
        self.output.push_front(line);
    }

    /// The command line is currently in use.
    /// Therefore no other os_inputs should be accepted.
    pub fn block(&self) -> bool {
        self.running
    }

    /// Get the text currently entered in the command line
    pub fn output(&self) -> Vec<String> {
        if self.running {
            let mut command = String::from("→");
            let mut hit_cursor = false;
            for (i, c) in self.command.char_indices() {
                if i == self.cursor {
                    hit_cursor = true;
                    command.push('■');
                }
                command.push(c);
            }
            if !hit_cursor {
                command.push('■');
            }

            let mut output = self.output.clone();
            output.insert(0, command);
            output.into()
        } else {
            vec!()
        }
    }
}
