use std::collections::HashMap;

use serde_json::{Value, Number};

pub fn engine_version() -> u64 { 3 }

pub fn engine_version_json() -> Value {
    Value::Number(Number::from_f64(engine_version() as f64).unwrap())
}

fn get_meta_engine_version(meta: &Option<Value>) -> u64 {
    if let &Some (ref meta) = meta {
        if let &Value::Object (ref object) = meta {
            if let Some (engine_version) = object.get("engine_version") {
                if let Some (value) = engine_version.as_u64() {
                    return value
                }
            }
        }
    }
    engine_version()
}

fn upgrade_meta_engine_version(meta: &mut Option<Value>) {
    if let &mut Some (ref mut meta) = meta {
        if let &mut Value::Object (ref mut object) = meta {
            object.insert(String::from("engine_version"), engine_version_json());
        }
    }
}

#[allow(unused_variables)]
pub fn upgrade_to_latest(meta: &mut Option<Value>, rules: &mut Option<Value>, fighters: &mut HashMap<String, Value>, stages: &mut Vec<Value>) {
    let meta_engine_version = get_meta_engine_version(meta);
    if meta_engine_version > engine_version() {
        panic!("Package is newer then this version of PF Sandbox. Please upgrade to the latest version.");
    }
    else if meta_engine_version < engine_version() {
        for upgrade_from in meta_engine_version..engine_version() {
            match upgrade_from {
                2 => { upgrade2(fighters) }
                1 => { upgrade1(fighters) }
                0 => { upgrade0(fighters) }
                _ => { }
            }
        }
        upgrade_meta_engine_version(meta);
    }
}

fn get_vec<'a>(parent: &'a mut Value, member: &str) -> Option<&'a mut Vec<Value>> {
    if let &mut Value::Object (ref mut object) = parent {
        if let Some (array) = object.get_mut(member) {
            if let &mut Value::Array (ref mut vector) = array {
                return Some (vector);
            }
        }
    }
    return None;
}

/// add force_hitlist_reset to ActionFrame
fn upgrade2(fighters: &mut HashMap<String, Value>) {
    for fighter in fighters.values_mut() {
        if let Some (actions) = get_vec(fighter, "actions") {
            for action in actions {
                if let Some (frames) = get_vec(action, "frames") {
                    for frame in frames {
                        if let &mut Value::Object (ref mut frame) = frame {
                            frame.insert(String::from("force_hitlist_reset"), Value::Bool(false));
                        }
                    }
                }
            }
        }
    }
}

/// add hitstun enum to hitboxes
fn upgrade1(fighters: &mut HashMap<String, Value>) {
    for fighter in fighters.values_mut() {
        if let Some (actions) = get_vec(fighter, "actions") {
            for action in actions {
                if let Some (frames) = get_vec(action, "frames") {
                    for frame in frames {
                        if let Some (colboxes) = get_vec(frame, "colboxes") {
                            for colbox in colboxes {
                                if let &mut Value::Object (ref mut colbox) = colbox {
                                    if let Some (role) = colbox.get_mut("role") {
                                        if let &mut Value::Object (ref mut role) = role {
                                            if let Some (hitbox) = role.get_mut("Hit") {
                                                if let &mut Value::Object (ref mut hitbox) = hitbox {
                                                    let hitstun = json!({"FramesTimesKnockback": 0.5});
                                                    hitbox.insert(String::from("hitstun"), hitstun);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Add order vec to frame
/// Change Meld into MeldFirst
fn upgrade0(fighters: &mut HashMap<String, Value>) {
    for fighter in fighters.values_mut() {
        if let Some (actions) = get_vec(fighter, "actions") {
            for action in actions {
                if let Some (frames) = get_vec(action, "frames") {
                    for frame in frames {
                        if let &mut Value::Object (ref mut frame) = frame {
                            frame.insert(String::from("render_order"), Value::Array(vec!()));
                        }

                        if let Some (colbox_links) = get_vec(frame, "colbox_links") {
                            for colbox_link in colbox_links {
                                if let &mut Value::Object (ref mut colbox_link) = colbox_link {
                                    let mut old_value = false;
                                    if let Some (link_type) = colbox_link.get_mut("link_type") {
                                        if let &mut Value::String (ref mut link_type_string) = link_type {
                                            if link_type_string.as_str() == "Meld" {
                                                old_value = true;
                                            }
                                        }
                                    }
                                    if old_value {
                                        colbox_link.insert(String::from("link_type"), Value::String(String::from("MeldFirst")));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
