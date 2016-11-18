#![feature(proc_macro)]

extern crate libusb;
extern crate num;
extern crate getopts;
extern crate treeflection;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate enum_primitive;
#[macro_use] extern crate glium;
#[macro_use] extern crate treeflection_derive;

pub mod app;
pub mod buffers;
pub mod camera;
pub mod cli;
pub mod command;
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
