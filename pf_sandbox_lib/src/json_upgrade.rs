use serde_json::{Value, Number};

pub fn build_version() -> String { String::from(env!("BUILD_VERSION")) }

pub fn engine_version() -> u64 { 15 }

pub fn engine_version_json() -> Value {
    Value::Number(Number::from(engine_version()))
}

fn get_engine_version(object: &Value) -> u64 {
    if let &Value::Object (ref object) = object {
        if let Some (engine_version) = object.get("engine_version") {
            if let Some (value) = engine_version.as_u64() {
                return value
            }
        }
    }
    engine_version()
}

fn upgrade_engine_version(meta: &mut Value) {
    if let &mut Value::Object (ref mut object) = meta {
        object.insert(String::from("engine_version"), engine_version_json());
    }
}

pub(crate) fn upgrade_to_latest_fighter(fighter: &mut Value, file_name: &str) {
    let fighter_engine_version = get_engine_version(fighter);
    if fighter_engine_version > engine_version() {
        println!("Fighter: {} is newer than this version of PF Sandbox. Please upgrade to the latest version.", file_name);
        // TODO: Display warning in window
    }
    else if fighter_engine_version < engine_version() {
        for upgrade_from in fighter_engine_version..engine_version() {
            match upgrade_from {
                14 => { upgrade_fighter14(fighter) }
                13 => { upgrade_fighter13(fighter) }
                12 => { upgrade_fighter12(fighter) }
                11 => { upgrade_fighter11(fighter) }
                10 => { upgrade_fighter10(fighter) }
                9  => { upgrade_fighter9(fighter) }
                8  => { upgrade_fighter8(fighter) }
                7  => { upgrade_fighter7(fighter) }
                6  => { upgrade_fighter6(fighter) }
                5  => { upgrade_fighter5(fighter) }
                4  => { upgrade_fighter4(fighter) }
                3  => { upgrade_fighter3(fighter) }
                2  => { upgrade_fighter2(fighter) }
                1  => { upgrade_fighter1(fighter) }
                0  => { upgrade_fighter0(fighter) }
                _ => { }
            }
        }
        upgrade_engine_version(fighter);
    }
}

pub(crate) fn upgrade_to_latest_stage(stage: &mut Value, file_name: &str) {
    let stage_engine_version = get_engine_version(stage);
    if stage_engine_version > engine_version() {
        println!("Stage: {} is newer than this version of PF Sandbox. Please upgrade to the latest version.", file_name);
        // TODO: Display warning in window
    }
    else if stage_engine_version < engine_version() {
        // TODO: Handle upgrades here
        upgrade_engine_version(stage);
    }
}

pub(crate) fn upgrade_to_latest_rules(rules: &mut Value) {
    let rules_engine_version = get_engine_version(rules);
    if rules_engine_version > engine_version() {
        println!("rules.json is newer than this version of PF Sandbox. Please upgrade to the latest version.");
        // TODO: Display warning in window
    }
    else if rules_engine_version < engine_version() {
        // TODO: Handle upgrades here
        upgrade_engine_version(rules);
    }
}

pub(crate) fn upgrade_to_latest_meta(meta: &mut Value) {
    let meta_engine_version = get_engine_version(meta);
    if meta_engine_version > engine_version() {
        println!("meta.json is newer than this version of PF Sandbox. Please upgrade to the latest version.");
        // TODO: Display warning in window
    }
    else if meta_engine_version < engine_version() {
        // TODO: Handle upgrades here
        upgrade_engine_version(meta);
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

// Important:
// Upgrades cannot rely on current structs as future changes may break those past upgrades
//
/// move set_x_vel/set_y_vel to x_vel_modify/y_vel_modify and x_vel_temp/y_vel_temp
fn upgrade_fighter14(fighter: &mut Value) {
    if let Some (actions) = get_vec(fighter, "actions") {
        for action in actions {
            if let Some (frames) = get_vec(action, "frames") {
                for frame in frames {
                    if let &mut Value::Object (ref mut frame) = frame {
                        frame.remove(&String::from("set_x_vel"));
                        frame.remove(&String::from("set_y_vel"));
                        frame.insert(String::from("x_vel_modify"), json!("None"));
                        frame.insert(String::from("y_vel_modify"), json!("None"));
                        frame.insert(String::from("x_vel_temp"), json!(0.0));
                        frame.insert(String::from("y_vel_temp"), json!(0.0));
                    }
                }
            }
        }
    }
}

/// Split spawn action into spawn and respawn
fn upgrade_fighter13(fighter: &mut Value) {
    let action = json!({
      "frames": [
        {
          "ecb": {
            "top": 16.0,
            "left": -4.0,
            "right": 4.0,
            "bottom": 0.0
          },
          "colboxes": [],
          "colbox_links": [],
          "render_order": [],
          "effects": [],
          "item_hold_x": 4.0,
          "item_hold_y": 11.0,
          "grab_hold_x": 4.0,
          "grab_hold_y": 11.0,
          "pass_through": true,
          "ledge_cancel": false,
          "use_platform_angle": false,
          "ledge_grab_box": null,
          "force_hitlist_reset": false
        }
      ],
      "iasa": 0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        actions.insert(0, action.clone());
    }
}

/// add enable_reverse_hit to Hit
fn upgrade_fighter12(fighter: &mut Value) {
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
                                                hitbox.insert(String::from("enable_reverse_hit"), Value::Bool(true));
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

/// Upgrade to new ECB format
fn upgrade_fighter11(fighter: &mut Value) {
    let ecb = json!({
        "top": 16.0,
        "left": -4.0,
        "right": 4.0,
        "bottom": 0.0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        for action in actions {
            if let Some (frames) = get_vec(action, "frames") {
                for frame in frames {
                    if let &mut Value::Object (ref mut frame) = frame {
                        frame.insert(String::from("ecb"), ecb.clone());
                    }
                }
            }
        }
    }
}

/// Change CSS properties
fn upgrade_fighter10(fighter: &mut Value) {
    if let &mut Value::Object (ref mut fighter) = fighter {
        fighter.insert(String::from("css_action"), Value::Number(Number::from(2)));
        fighter.insert(String::from("css_scale"), Value::Number(Number::from(1)));
    }
}

/// Add turn properties to fighter
fn upgrade_fighter9(fighter: &mut Value) {
    if let &mut Value::Object (ref mut fighter) = fighter {
        fighter.insert(String::from("run_turn_flip_dir_frame"), Value::Number(Number::from(30)));
        fighter.insert(String::from("tilt_turn_flip_dir_frame"), Value::Number(Number::from(5)));
        fighter.insert(String::from("tilt_turn_into_dash_iasa"), Value::Number(Number::from(5)));
    }
}

/// Add use_platform_angle to ActionFrame
fn upgrade_fighter8(fighter: &mut Value) {
    if let Some (actions) = get_vec(fighter, "actions") {
        for action in actions {
            if let Some (frames) = get_vec(action, "frames") {
                for frame in frames {
                    if let &mut Value::Object (ref mut frame) = frame {
                        frame.insert(String::from("use_platform_angle"), Value::Bool(false));
                    }
                }
            }
        }
    }
}

/// Add MissedTech states
fn upgrade_fighter7(fighter: &mut Value) {
    let action_indexes: Vec<usize> = vec!(7, 46, 47, 48, 54, 69);
    let action = json!({
      "frames": [
        {
          "ecb": {
            "top_x": 0.0,
            "top_y": 16.0,
            "left_x": -4.0,
            "left_y": 11.0,
            "right_x": 4.0,
            "right_y": 11.0,
            "bot_x": 0.0,
            "bot_y": 0.0
          },
          "colboxes": [],
          "colbox_links": [],
          "render_order": [],
          "effects": [],
          "item_hold_x": 4.0,
          "item_hold_y": 11.0,
          "grab_hold_x": 4.0,
          "grab_hold_y": 11.0,
          "pass_through": true,
          "ledge_cancel": false,
          "ledge_grab_box": null,
          "force_hitlist_reset": false
        }
      ],
      "iasa": 0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        for action_index in &action_indexes {
            actions.insert(*action_index, action.clone());
        }
    }
}

/// Add power shield state
fn upgrade_fighter6(fighter: &mut Value) {
    let action_indexes: Vec<usize> = vec!(31, 47, 48, 49);
    let action = json!({
      "frames": [
        {
          "ecb": {
            "top_x": 0.0,
            "top_y": 16.0,
            "left_x": -4.0,
            "left_y": 11.0,
            "right_x": 4.0,
            "right_y": 11.0,
            "bot_x": 0.0,
            "bot_y": 0.0
          },
          "colboxes": [],
          "colbox_links": [],
          "render_order": [],
          "effects": [],
          "item_hold_x": 4.0,
          "item_hold_y": 11.0,
          "grab_hold_x": 4.0,
          "grab_hold_y": 11.0,
          "pass_through": true,
          "ledge_cancel": false,
          "ledge_grab_box": null,
          "force_hitlist_reset": false
        }
      ],
      "iasa": 0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        for action_index in &action_indexes {
            actions.insert(*action_index, action.clone());
        }
    }
}

/// teeter + ledge cancel
fn upgrade_fighter5(fighter: &mut Value) {
    //ledge_cancel to ActionFrame
    if let Some (actions) = get_vec(fighter, "actions") {
        for action in actions {
            if let Some (frames) = get_vec(action, "frames") {
                for frame in frames {
                    if let &mut Value::Object (ref mut frame) = frame {
                        frame.insert(String::from("ledge_cancel"), Value::Bool(true));
                    }
                }
            }
        }
    }

    // add teeter and spotdoge actions
    let action_indexes: Vec<usize> = vec!(5, 6, 36);
    let action = json!({
      "frames": [
        {
          "ecb": {
            "top_x": 0.0,
            "top_y": 16.0,
            "left_x": -4.0,
            "left_y": 11.0,
            "right_x": 4.0,
            "right_y": 11.0,
            "bot_x": 0.0,
            "bot_y": 0.0
          },
          "colboxes": [],
          "colbox_links": [],
          "render_order": [],
          "effects": [],
          "item_hold_x": 4.0,
          "item_hold_y": 11.0,
          "grab_hold_x": 4.0,
          "grab_hold_y": 11.0,
          "pass_through": true,
          "ledge_cancel": false,
          "ledge_grab_box": null,
          "force_hitlist_reset": false
        }
      ],
      "iasa": 0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        for action_index in &action_indexes {
            actions.insert(*action_index, action.clone());
        }
    }
}

/// add ledge actions
fn upgrade_fighter4(fighter: &mut Value) {
    let action_indexes: Vec<usize> = vec!(4, 24, 25, 26, 27, 28, 41, 42, 55, 56);
    let action = json!({
      "frames": [
        {
          "ecb": {
            "top_x": 0.0,
            "top_y": 16.0,
            "left_x": -4.0,
            "left_y": 11.0,
            "right_x": 4.0,
            "right_y": 11.0,
            "bot_x": 0.0,
            "bot_y": 0.0
          },
          "colboxes": [],
          "colbox_links": [],
          "render_order": [],
          "effects": [],
          "item_hold_x": 4.0,
          "item_hold_y": 11.0,
          "grab_hold_x": 4.0,
          "grab_hold_y": 11.0,
          "pass_through": true,
          "ledge_grab_box": null,
          "force_hitlist_reset": false
        }
      ],
      "iasa": 0
    });

    if let Some (actions) = get_vec(fighter, "actions") {
        for action_index in &action_indexes {
            actions.insert(*action_index, action.clone());
        }
    }
}

/// add pass_through to ActionFrame
fn upgrade_fighter3(fighter: &mut Value) {
    if let Some (actions) = get_vec(fighter, "actions") {
        for action in actions {
            if let Some (frames) = get_vec(action, "frames") {
                for frame in frames {
                    if let &mut Value::Object (ref mut frame) = frame {
                        frame.insert(String::from("pass_through"), Value::Bool(true));
                    }
                }
            }
        }
    }
}

/// add force_hitlist_reset to ActionFrame
fn upgrade_fighter2(fighter: &mut Value) {
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

/// add hitstun enum to hitboxes
fn upgrade_fighter1(fighter: &mut Value) {
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

/// Add order vec to frame
/// Change Meld into MeldFirst
fn upgrade_fighter0(fighter: &mut Value) {
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
