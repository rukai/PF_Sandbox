#![windows_subsystem = "windows"]

use gtk::prelude::*;
use gtk::{
    Box,
    Button,
    Label,
    Orientation,
    Window,
    WindowType,
};

fn main() {
    gtk::init().unwrap();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("PF TAS");

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    window.add(&vbox);

    let label_text = format!("Uhhh ... I kind of deleted the tas tool because it was garbage and needed to be rewritten and I didnt want to maintain the existing tool anymore. Sorry...");
    let label = Label::new(Some(label_text.as_ref()));
    vbox.add(&label);

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
