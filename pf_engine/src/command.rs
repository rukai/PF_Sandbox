use package::Package;
use node::{Node, NodeRunner};

/// Run the passed command
/// Return any output it generates

// TODO: Turn this into a Node that manages packages
// Or get a vec to do it for us or something ...
pub fn run(command: &str, package: &mut Package) -> String {
    match NodeRunner::new(command) {
        Ok(runner) => package.node_step(runner),
        Err(msg)   => msg
    }
}
