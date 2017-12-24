extern crate gdk;
extern crate gtk;
extern crate gilrs;
extern crate serde_json;
extern crate pf_sandbox;
extern crate uuid;

mod state;

use uuid::Uuid;
use gtk::prelude::*;
use gdk::Atom;
use gtk::{
    Box,
    Button,
    CheckButton,
    Clipboard,
    ComboBoxText,
    Entry,
    EntryBuffer,
    Label,
    Orientation,
    PolicyType,
    ScrolledWindow,
    Window,
    WindowType,
};
use gilrs::EventType;

use pf_sandbox::input::maps::{
    AnalogDest,
    DigitalDest,
    AnalogMap,
    DigitalMap,
    AnalogFilter,
    DigitalFilter,
    OS
};

use state::{State, Code, AnalogHistory};

use std::rc::Rc;
use std::sync::RwLock;

// make moving clones into closures more convenient
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn main() {
    // Need to be careful with the rw lock.
    // It is easy to accidentally create a deadlock by accidentally triggering
    // a write locking callback, while we have a read lock, or vice versa.
    let state = Rc::new(RwLock::new(State::new()));

    gtk::init().unwrap();

    let window = Window::new(WindowType::Toplevel);
    window.set_property_default_height(800);
    window.set_title("PFS Controller Mapper");

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(10);
    vbox.set_margin_top(10);
    vbox.add(&save_copy_hbox(state.clone()));
    window.add(&vbox);

    let scrolled_window = ScrolledWindow::new(None, None);
    scrolled_window.set_property_hscrollbar_policy(PolicyType::Never);

    let inputs_vbox = Box::new(Orientation::Vertical, 20);
    inputs_vbox.set_margin_top(10);
    inputs_vbox.set_margin_bottom(10);
    inputs_vbox.set_margin_right(10);
    scrolled_window.add_with_viewport(&inputs_vbox);

    let controller_select = controller_select_hbox(state.clone(), inputs_vbox.clone());
    controller_select.set_margin_right(10);
    vbox.add(&controller_select);

    vbox.add(&input_management_hbox(state.clone()));

    vbox.pack_end(&scrolled_window, true, true, 0);

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}

fn save_copy_hbox(state: Rc<RwLock<State>>) -> Box {
    let hbox = Box::new(Orientation::Horizontal, 5);

    let save = Button::new_with_label("Save");
    save.connect_clicked(clone!(state => move |_| {
        let state = state.read().unwrap();
        state.controller_maps.save();
    }));
    hbox.add(&save);

    let copy = Button::new_with_label("Copy JSON to clipboard");
    copy.connect_clicked(clone!(state => move |_| {
        let state = state.read().unwrap();
        if let Some(controller_map) = state.controller {
            let json = serde_json::to_string_pretty(&state.controller_maps.maps[controller_map]).unwrap();
            Clipboard::get(&Atom::from("CLIPBOARD")).set_text(json.as_ref());
        }
    }));
    hbox.add(&copy);

    hbox
}

fn controller_select_hbox(state: Rc<RwLock<State>>, inputs_vbox: Box) -> Box {
    let hbox = Box::new(Orientation::Horizontal, 5);

    let combo_box = ComboBoxText::new();
    combo_box.connect_changed(clone!(state, combo_box => move |_| {
        let mut controller = None;
        if let Some(text) = combo_box.get_active_text() {
            let mut state = state.write().unwrap();
            for (i, controller_map) in state.controller_maps.maps.iter().enumerate() {
                if controller_map.name == text && controller_map.os == OS::get_current() {
                    controller = Some(i);
                }
            }
            state.controller = controller;
            state.ui_to_digital_map.clear();
            state.ui_to_analog_map.clear();
        }
        populate_inputs(state.clone(), inputs_vbox.clone());
    }));
    hbox.add(&combo_box);

    let only_plugged_in = CheckButton::new_with_label("Only show plugged in controllers");
    only_plugged_in.connect_toggled(clone!(state, only_plugged_in, combo_box => move |_| {
        combo_box.remove_all();
        let state = state.read().unwrap();
        if only_plugged_in.get_active() {
            for map in state.controller_maps.maps.iter() {
                let mut add = false;
                for (_, gamepad) in state.gilrs.gamepads() {
                    if gamepad.name() == map.name && OS::get_current() == map.os {
                        add = true;
                    }
                }
                if add {
                    combo_box.append_text(map.name.as_ref());
                }
            }
        }
        else {
            combo_box.remove_all();
            for map in state.controller_maps.maps.iter() {
                if OS::get_current() == map.os {
                    combo_box.append_text(map.name.as_ref());
                }
            }
        }
    }));
    only_plugged_in.set_active(true);
    hbox.add(&only_plugged_in);

    hbox
}

fn input_management_hbox(state: Rc<RwLock<State>>) -> Box {
    let hbox = Box::new(Orientation::Horizontal, 5);
    hbox.add(&Label::new(Some("Last Input")));

    let label = Label::new(Some(""));

    let refresh_button = Button::new_with_label("Clear");
    refresh_button.connect_clicked(clone!(state, label => move |_| {
        let mut state = state.write().unwrap();
        state.last_code = Code::None;
        state.analog_history.clear();
        label.set_label("");
    }));
    hbox.add(&refresh_button);

    hbox.add(&label);

    gtk::timeout_add(16, move || {
        let mut state = state.write().unwrap();
        while let Some(ev) = state.gilrs.next_event() {
            match ev.event {
                EventType::ButtonPressed (_, code) => {
                    state.last_code = Code::Digital(code as usize);
                    label.set_text(format!("Digital: {}", code).as_ref());
                }
                EventType::AxisChanged (_, value, code) => {
                    let code = code as usize;
                    let new_history = match state.analog_history.get(&code).cloned() {
                        Some(mut history) => {
                            if value > history.max {
                                history.max = value;
                            }
                            if value < history.min {
                                history.min = value;
                            }
                            history.last_value = value;
                            history
                        }
                        None => {
                            AnalogHistory::new(value)
                        }
                    };
                    state.analog_history.insert(code, new_history.clone());

                    let mut update = true;
                    for history in state.analog_history.values() {
                        if value > history.min && value < history.max {
                            update = false;
                        }
                    }
                    if update {
                        state.last_code = Code::Analog(code, new_history);
                        label.set_text(format!("Analog: {} Value: {}", code, value).as_ref());
                    }
                }
                _ => { }
            }
        }
        Continue(true)
    });

    hbox
}

fn populate_inputs(state: Rc<RwLock<State>>, vbox: Box) {
    for children in vbox.get_children() {
        vbox.remove(&children);
    }

    if state.read().unwrap().controller.is_some() {
        vbox.add(&digital_input_vbox(state.clone(), String::from("A Button"),     DigitalDest::A));
        vbox.add(&digital_input_vbox(state.clone(), String::from("B Button"),     DigitalDest::B));
        vbox.add(&digital_input_vbox(state.clone(), String::from("X Button"),     DigitalDest::X));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Y Button"),     DigitalDest::Y));
        vbox.add(&digital_input_vbox(state.clone(), String::from("L Button"),     DigitalDest::L));
        vbox.add(&digital_input_vbox(state.clone(), String::from("R Button"),     DigitalDest::R));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Z Button"),     DigitalDest::Z));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Start Button"), DigitalDest::Start));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Left DPAD"),    DigitalDest::Left));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Right DPAD"),   DigitalDest::Right));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Up DPAD"),      DigitalDest::Up));
        vbox.add(&digital_input_vbox(state.clone(), String::from("Down DPAD"),    DigitalDest::Down));

        vbox.add(&analog_input_vbox(state.clone(), String::from("Horizontal Main Stick"), AnalogDest::StickX));
        vbox.add(&analog_input_vbox(state.clone(), String::from("Vertical Main Stick"),   AnalogDest::StickY));
        vbox.add(&analog_input_vbox(state.clone(), String::from("Horizontal C Stick"),    AnalogDest::CStickX));
        vbox.add(&analog_input_vbox(state.clone(), String::from("Vertical C Stick"),      AnalogDest::CStickY));
        vbox.add(&analog_input_vbox(state.clone(), String::from("Left Trigger"),          AnalogDest::LTrigger));
        vbox.add(&analog_input_vbox(state,         String::from("Right Trigger"),         AnalogDest::RTrigger));
        vbox.show_all();
    }
}

/* Digital Input UI */

fn digital_input_vbox(state: Rc<RwLock<State>>, input_text: String, dest: DigitalDest) -> Box {
    let vbox = Box::new(Orientation::Vertical, 5);

    let input_gc = digital_input_gc_hbox(state.clone(), vbox.clone(), input_text, dest.clone());
    vbox.add(&input_gc);

    let controller = state.read().unwrap().controller;
    if let Some(controller) = controller {
        let maps = state.read().unwrap().controller_maps.maps[controller].get_digital_maps(dest);
        for (index, map) in maps {
            let input_map = input_digital_map_hbox(state.clone(), map, index);
            vbox.add(&input_map);
        }
    }

    vbox
}

fn digital_input_gc_hbox(state: Rc<RwLock<State>>, vbox: Box, input_text: String, dest: DigitalDest) -> Box {
    let hbox = Box::new(Orientation::Horizontal, 5);

    let input_label = Label::new(Some(input_text.as_str()));
    input_label.set_property_xalign(0.0);
    hbox.add(&input_label);

    let detect_button = Button::new_with_label("Add from last input");
    detect_button.connect_clicked(clone!(state, vbox, dest => move |_| {
        let last_code = state.read().unwrap().last_code.clone();
        match last_code {
            Code::Analog (code, history) => {
                let map = DigitalMap {
                    dest:   dest.clone(),
                    filter: DigitalFilter::FromAnalog { min: history.last_value, max: history.last_value },
                    source: code,
                };
                new_input_digital_map(state.clone(), vbox.clone(), map);
            }
            Code::Digital (code) => {
                let map = DigitalMap {
                    dest:   dest.clone(),
                    filter: DigitalFilter::FromDigital,
                    source: code,
                };
                new_input_digital_map(state.clone(), vbox.clone(), map);
            }
            Code::None => { }
        }
    }));
    hbox.add(&detect_button);

    let add_digital_button = Button::new_with_label("Add empty Digital");
    add_digital_button.connect_clicked(clone!(state, vbox, dest => move |_| {
        let map = DigitalMap {
            source: 0,
            dest:   dest.clone(),
            filter: DigitalFilter::default_digital(),
        };
        new_input_digital_map(state.clone(), vbox.clone(), map);
    }));
    hbox.add(&add_digital_button);

    let add_analog_button = Button::new_with_label("Add empty Analog");
    add_analog_button.connect_clicked(move |_| {
        let map = DigitalMap {
            source: 0,
            dest:   dest.clone(),
            filter: DigitalFilter::default_analog()
        };
        new_input_digital_map(state.clone(), vbox.clone(), map);
    });
    hbox.add(&add_analog_button);

    hbox
}

fn new_input_digital_map(state: Rc<RwLock<State>>, vbox: Box, map: DigitalMap) {
    let push_index = {
        let mut state = state.write().unwrap();
        let i = state.controller.unwrap();
        state.controller_maps.maps[i].digital_maps.push(map.clone());
        state.controller_maps.maps[i].digital_maps.len() - 1
    };

    let input_map = input_digital_map_hbox(state.clone(), map, push_index);
    vbox.add(&input_map);
    vbox.show_all();
}

fn input_digital_map_hbox(state: Rc<RwLock<State>>, digital_map: DigitalMap, index: usize) -> Box {
    let uuid = Uuid::new_v4();
    state.write().unwrap().ui_to_digital_map.insert(uuid, index);

    let hbox = Box::new(Orientation::Horizontal, 5);
    hbox.set_margin_start(60);

    hbox.add(&Label::new(Some(if digital_map.filter.is_digital_source() { "Digital code" } else { "Analog code" })));

    let input_code = digital_map.source.to_string();
    let code_entry_buffer = EntryBuffer::new(Some(input_code.as_ref()));
    let code_entry = Entry::new_with_buffer(&code_entry_buffer);
    code_entry.connect_changed(clone!(state => move |_| {
        if let Ok(value) = code_entry_buffer.get_text().parse() {
            let mut state = state.write().unwrap();
            let map_i = state.controller.unwrap();
            let digital_map_i = state.ui_to_digital_map[&uuid];
            state.controller_maps.maps[map_i].digital_maps[digital_map_i].source = value;
        }
    }));
    hbox.add(&code_entry);

    match digital_map.filter {
        DigitalFilter::FromAnalog { min, max } => {
            hbox.add(&Label::new(Some("min: ")));
            let min_entry_buffer = EntryBuffer::new(Some(min.to_string().as_ref()));
            let min_entry = Entry::new_with_buffer(&min_entry_buffer);
            min_entry.connect_changed(clone!(state => move |_| {
                if let Ok(value) = min_entry_buffer.get_text().parse() {
                    let mut state = state.write().unwrap();
                    let map_i = state.controller.unwrap();
                    let digital_map_i = state.ui_to_digital_map[&uuid];
                    state.controller_maps.maps[map_i].digital_maps[digital_map_i].filter.set_min(value);
                }
            }));
            hbox.add(&min_entry);

            hbox.add(&Label::new(Some("max: ")));
            let max_entry_buffer = EntryBuffer::new(Some(max.to_string().as_ref()));
            let max_entry = Entry::new_with_buffer(&max_entry_buffer);
            max_entry.connect_changed(clone!(state => move |_| {
                if let Ok(value) = max_entry_buffer.get_text().parse() {
                    let mut state = state.write().unwrap();
                    let map_i = state.controller.unwrap();
                    let digital_map_i = state.ui_to_digital_map[&uuid];
                    state.controller_maps.maps[map_i].digital_maps[digital_map_i].filter.set_max(value);
                }
            }));
            hbox.add(&max_entry);
        }
        DigitalFilter::FromDigital => { }
    }

    let button = Button::new_with_label("Remove");
    button.connect_clicked(clone!(hbox => move |_| {
        // remove from UI
        hbox.destroy();

        // remove from map
        let mut state = state.write().unwrap();
        let map_i = state.controller.unwrap();
        let digital_map_i = state.ui_to_digital_map[&uuid];
        state.controller_maps.maps[map_i].digital_maps.remove(digital_map_i);

        // shift down ui_to_digital_map
        for index in state.ui_to_digital_map.values_mut() {
            if *index > digital_map_i {
                *index -= 1;
            }
        }
    }));
    hbox.add(&button);

    hbox
}

/* Analog Input UI */

fn analog_input_vbox(state: Rc<RwLock<State>>, input_text: String, dest: AnalogDest) -> Box {
    let vbox = Box::new(Orientation::Vertical, 5);

    let input_gc = analog_input_gc_hbox(state.clone(), vbox.clone(), input_text, dest.clone());
    vbox.add(&input_gc);

    let controller = state.read().unwrap().controller;
    if let Some(controller) = controller {
        let maps = state.read().unwrap().controller_maps.maps[controller].get_analog_maps(dest);
        for (index, map) in maps {
            let input_map = input_analog_map_hbox(state.clone(), map, index);
            vbox.add(&input_map);
        }
    }

    vbox
}

fn analog_input_gc_hbox(state: Rc<RwLock<State>>, vbox: Box, input_text: String, dest: AnalogDest) -> Box {
    let hbox = Box::new(Orientation::Horizontal, 5);

    let input_label = Label::new(Some(input_text.as_str()));
    input_label.set_property_xalign(0.0);
    hbox.add(&input_label);

    let detect_button = Button::new_with_label("Add from last input");
    detect_button.connect_clicked(clone!(state, vbox, dest => move |_| {
        let last_code = state.read().unwrap().last_code.clone();
        match last_code {
            Code::Analog (code, history) => {
                let map = AnalogMap {
                    dest:   dest.clone(),
                    filter: AnalogFilter::FromAnalog { min: history.min, max: history.max, flip: false },
                    source: code,
                };
                new_input_analog_map(state.clone(), vbox.clone(), map);
            }
            Code::Digital (code) => {
                let map = AnalogMap {
                    dest:   dest.clone(),
                    filter: AnalogFilter::default_digital(),
                    source: code,
                };
                new_input_analog_map(state.clone(), vbox.clone(), map);
            }
            Code::None => { }
        }
    }));
    hbox.add(&detect_button);

    let add_digital_button = Button::new_with_label("Add empty Digital");
    add_digital_button.connect_clicked(clone!(state, vbox, dest => move |_| {
        let map = AnalogMap {
            source: 0,
            dest:   dest.clone(),
            filter: AnalogFilter::default_digital(),
        };
        new_input_analog_map(state.clone(), vbox.clone(), map);
    }));
    hbox.add(&add_digital_button);

    let add_analog_button = Button::new_with_label("Add empty Analog");
    add_analog_button.connect_clicked(move |_| {
        let map = AnalogMap {
            source: 0,
            dest:   dest.clone(),
            filter: AnalogFilter::default_analog(),
        };
        new_input_analog_map(state.clone(), vbox.clone(), map);
    });
    hbox.add(&add_analog_button);

    hbox
}

fn new_input_analog_map(state: Rc<RwLock<State>>, vbox: Box, map: AnalogMap) {
    let push_index = {
        let mut state = state.write().unwrap();
        let i = state.controller.unwrap();
        state.controller_maps.maps[i].analog_maps.push(map.clone());
        state.controller_maps.maps[i].analog_maps.len() - 1
    };

    let input_map = input_analog_map_hbox(state.clone(), map, push_index);
    vbox.add(&input_map);
    vbox.show_all();
}

fn input_analog_map_hbox(state: Rc<RwLock<State>>, analog_map: AnalogMap, index: usize) -> Box {
    let uuid = Uuid::new_v4();
    state.write().unwrap().ui_to_analog_map.insert(uuid, index);

    let hbox = Box::new(Orientation::Horizontal, 5);
    hbox.set_margin_start(60);

    hbox.add(&Label::new(Some(if analog_map.filter.is_digital_source() { "Digital code" } else { "Analog code" })));

    let input_code = analog_map.source.to_string();
    let code_entry_buffer = EntryBuffer::new(Some(input_code.as_ref()));
    let code_entry = Entry::new_with_buffer(&code_entry_buffer);
    code_entry.connect_changed(clone!(state => move |_| {
        if let Ok(value) = code_entry_buffer.get_text().parse() {
            let mut state = state.write().unwrap();
            let map_i = state.controller.unwrap();
            let analog_map_i = state.ui_to_analog_map[&uuid];
            state.controller_maps.maps[map_i].analog_maps[analog_map_i].source = value;
        }
    }));
    hbox.add(&code_entry);

    match analog_map.filter {
        AnalogFilter::FromAnalog { min, max, flip } => {
            hbox.add(&Label::new(Some("min: ")));
            let min_entry_buffer = EntryBuffer::new(Some(min.to_string().as_ref()));
            let min_entry = Entry::new_with_buffer(&min_entry_buffer);
            min_entry.connect_changed(clone!(state => move |_| {
                if let Ok(value) = min_entry_buffer.get_text().parse() {
                    let mut state = state.write().unwrap();
                    let map_i = state.controller.unwrap();
                    let analog_map_i = state.ui_to_analog_map[&uuid];
                    state.controller_maps.maps[map_i].analog_maps[analog_map_i].filter.set_min(value);
                }
            }));
            hbox.add(&min_entry);

            hbox.add(&Label::new(Some("max: ")));
            let max_entry_buffer = EntryBuffer::new(Some(max.to_string().as_ref()));
            let max_entry = Entry::new_with_buffer(&max_entry_buffer);
            max_entry.connect_changed(clone!(state => move |_| {
                if let Ok(value) = max_entry_buffer.get_text().parse() {
                    let mut state = state.write().unwrap();
                    let map_i = state.controller.unwrap();
                    let analog_map_i = state.ui_to_analog_map[&uuid];
                    state.controller_maps.maps[map_i].analog_maps[analog_map_i].filter.set_max(value);
                }
            }));
            hbox.add(&max_entry);

            let flip_check_button = CheckButton::new_with_label("flip: ");
            flip_check_button.connect_toggled(clone!(state, flip_check_button => move |_| {
                let mut state = state.write().unwrap();
                let map_i = state.controller.unwrap();
                let analog_map_i = state.ui_to_analog_map[&uuid];
                state.controller_maps.maps[map_i].analog_maps[analog_map_i].filter.set_flip(flip_check_button.get_active());
            }));
            flip_check_button.set_active(flip);
            hbox.add(&flip_check_button);
        }

        AnalogFilter::FromDigital { value } => {
            hbox.add(&Label::new(Some("value: ")));
            let value_entry_buffer = EntryBuffer::new(Some(value.to_string().as_ref()));
            let value_entry = Entry::new_with_buffer(&value_entry_buffer);
            value_entry.connect_changed(clone!(state => move |_| {
                if let Ok(value) = value_entry_buffer.get_text().parse() {
                    let mut state = state.write().unwrap();
                    let map_i = state.controller.unwrap();
                    let analog_map_i = state.ui_to_analog_map[&uuid];
                    state.controller_maps.maps[map_i].analog_maps[analog_map_i].filter.set_value(value);
                }
            }));
            hbox.add(&value_entry);
        }
    }

    let button = Button::new_with_label("Remove");
    button.connect_clicked(clone!(hbox => move |_| {
        // remove from ui
        hbox.destroy();

        // remove from map
        let mut state = state.write().unwrap();
        let map_i = state.controller.unwrap();
        let analog_map_i = state.ui_to_analog_map[&uuid];
        state.controller_maps.maps[map_i].analog_maps.remove(analog_map_i);

        // shift down ui_to_analog_map
        for index in state.ui_to_analog_map.values_mut() {
            if *index > analog_map_i {
                *index -= 1;
            }
        }
    }));
    hbox.add(&button);

    hbox
}
