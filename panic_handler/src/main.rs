#![windows_subsystem = "windows"]

use pf_sandbox_lib::panic_handler::Report;

use std::env;

use gtk::prelude::*;
use gtk::{
    Box,
    Button,
    Label,
    LinkButton,
    Orientation,
    Window,
    WindowType,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Some(file_name) = args.get(1) {
        display_window(file_name);
    }
    else {
        // The user probably just manually ran the executable dont tell them to report anything
    }
}

fn display_window(file_name: &str) {
    gtk::init().unwrap();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Panic Handler");

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    window.add(&vbox);

    match Report::from_file(file_name) {
        Ok (report) => {
            let hbox = Box::new(Orientation::Horizontal, 5);
            vbox.add(&hbox);
            let label = Label::new(format!("{} has panicked.\nPlease report this at:", report.crate_name).as_ref());
            hbox.add(&label);

            let hbox = Box::new(Orientation::Horizontal, 5);
            vbox.add(&hbox);

            let title = match (report.payload, report.location_file, report.location_line) {
                (Some(payload), Some(file), Some(line)) => format!("{}:{} {:.100}", file, line, payload),
                (None,          Some(file), Some(line)) => format!("{}:{} UNKNOWN PANIC", file, line),
                (None,          Some(file), None      ) => format!("{} UNKNOWN PANIC", file),
                (Some(payload), _,          _         ) => format!("{:.100}", payload),
                (_,             _,          _         ) => String::from("PLEASE DESCRIBE CAUSE"),
            };
            let body = "PLEASE REPLACE THIS TEXT WITH WHAT YOU WERE DOING WHEN THE PANIC OCCURRED.%0A%0APLEASE ENSURE THE PANIC DUMP FILE IS ATTACHED.";
            let address = format!("https://github.com/rukai/PF_Sandbox/issues/new?labels=panic&title={}&body={}", title, body);
            let link = LinkButton::new_with_label(&address, "https://github.com/rukai/PF_Sandbox/issues/new");
            if address.len() >= 2000 {
                eprintln!("This address is too long, it may not work on some browsers:\n{}", address);
            }
            hbox.add(&link);

            let hbox = Box::new(Orientation::Horizontal, 5);
            vbox.add(&hbox);
            let label = Label::new(format!("Please attach this file to your issue report: {}", file_name).as_ref());
            hbox.add(&label);
        }
        Err (err) => {
            let label_text = format!("Failed to read the panic report:\n{}", err);
            let label = Label::new(label_text.as_ref());
            vbox.add(&label);
        }
    }

    let hbox = Box::new(Orientation::Horizontal, 5);
    vbox.add(&hbox);
    let button = Button::new_with_label("Close");
    button.connect_clicked(|_| {
        gtk::main_quit();
    });
    hbox.add(&button);

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}
