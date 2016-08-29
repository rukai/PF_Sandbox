use package::Package;

/// Run the passed command
/// Return any output it generates
pub fn run(command: &str, package: &Package) -> String {
    let mut sections = command.split_whitespace();

    let attributes = match sections.next() {
        Some(attribute_string) => {
            attribute_string.split('.').map(|s| s.to_string()).collect::<Vec<_>>()
        }
        None => { return String::from("Error: No attributes\n"); }
    };

    let action = match sections.next() {
        Some(action_string) => { action_string.clone() }
        None                => { return String::from("Error: No action\n"); }
    };

    let arguments = sections.map(|s| s.to_string()).collect::<Vec<_>>();

    println!("attributes: {:?}", attributes);
    println!("action: {:?}", action);
    println!("arguments: {:?}", arguments);

    // TODO: Get object via reflection
    //let object = package;
    //for attribute in attributes {
        //object = object.get_attribute(attributes);
    //}

    // TODO: Perform action via reflection
    //object.run_action(action, arguments)
    String::new()
}

trait PFObject {
    fn get_attribute<T: PFObject>(attribute: &str) -> T;
    fn set_attribute<T: PFObject>(attribute: &str, value: T);
    fn run_action(action: &str) -> String;
}
