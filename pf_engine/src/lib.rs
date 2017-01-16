#![feature(drop_types_in_const)]
extern crate getopts;
extern crate libusb;
extern crate num;
extern crate serde;
extern crate serde_json;
extern crate treeflection;
#[macro_use] extern crate enum_primitive;
#[macro_use] extern crate glium;
#[macro_use] extern crate matches;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate treeflection_derive;

pub mod app;
pub mod buffers;
pub mod camera;
pub mod cli;
pub mod collision;
pub mod fighter;
pub mod game;
pub mod graphics;
pub mod input;
pub mod menu;
pub mod network;
pub mod os_input;
pub mod package;
pub mod player;
pub mod rules;
pub mod stage;
