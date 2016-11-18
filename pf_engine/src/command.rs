use treeflection::{Node, NodeRunner};

use package::Package;

/// Run the passed command
/// Return any output it generates

// TODO: Turn this into a Node that manages packages
// Or get a vec to do it for us or something ...
pub fn run(command: &str, package: &mut Package) -> String {
    match NodeRunner::new(command) {
        Ok(runner) => {
            let result = package.node_step(runner);
            package.force_update_entire_package();
            result
        },
        Err(msg)   => msg
    }
}
