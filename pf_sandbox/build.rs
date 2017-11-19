use std::process::Command;

fn main() {
    let mut cmd = Command::new("git");
    cmd.args(&["describe", "--always", "--long", "--dirty"]);

    let version = if let Ok(output) = cmd.output() {
        if output.status.success() {
            String::from_utf8(output.stdout).unwrap_or(String::from("NO GIT"))
        } else {
            String::from("NO GIT")
        }
    } else {
        String::from("NO GIT")
    };
    println!("cargo:rustc-env=BUILD_VERSION={}", version.trim());
}
