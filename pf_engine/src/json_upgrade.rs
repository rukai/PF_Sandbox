use serde_json::Value;

pub fn engine_version() -> u64 {
    return 1;
}

fn get_meta_engine_version(meta: &Value) -> u64 {
    if let &Value::Object (ref object) = meta {
        if let Some (engine_version) = object.get("engine_version") {
            if let Some (value) = engine_version.as_u64() {
                return value
            }
        }
    }
    panic!("Invalid meta.json")
}

fn upgrade_meta_engine_version(meta: &mut Value) {
    if let &mut Value::Object (ref mut object) = meta {
        object.insert(String::from("engine_version"), Value::U64 (engine_version()));
    }
}

#[allow(unused_variables)]
pub fn upgrade_to_latest(meta: &mut Value, rules: &mut Value, fighters: &mut Vec<Value>, stages: &mut Vec<Value>) {
    let meta_engine_version = get_meta_engine_version(meta);
    if meta_engine_version > engine_version() {
        panic!("Package is newer then this version of PF Engine. Please upgrade to the latest version.");
    }
    else if meta_engine_version < engine_version() {
        for upgrade_from in meta_engine_version..engine_version() {
            match upgrade_from {
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

/// Add order vec to frame
/// Change Meld into MeldFirst
fn upgrade0(fighters: &mut Vec<Value>) {
    for fighter in fighters {
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
