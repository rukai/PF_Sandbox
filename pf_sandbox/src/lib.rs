#![feature(drop_types_in_const)]
extern crate chrono;
extern crate crypto;
extern crate getopts;
extern crate libusb;
extern crate num;
extern crate reqwest;
extern crate serde;
extern crate treeflection;
extern crate winit;
extern crate zip;
#[macro_use] extern crate enum_primitive;
#[macro_use] extern crate matches;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate treeflection_derive;

pub mod app;
pub mod camera;
pub mod cli;
pub mod collision;
pub mod config;
pub mod fighter;
pub mod files;
pub mod game;
pub mod graphics;
pub mod input;
pub mod json_upgrade;
pub mod menu;
pub mod network;
pub mod os_input;
pub mod package;
pub mod player;
pub mod records;
pub mod replays;
pub mod rules;
pub mod stage;
pub mod math;

#[cfg(feature = "vulkan")]
#[macro_use]
extern crate vulkano;
#[cfg(feature = "vulkan")]
extern crate vulkano_win;
#[cfg(feature = "vulkan")]
extern crate vulkano_text;
#[cfg(feature = "vulkan")]
pub mod vulkan;
#[cfg(feature = "vulkan")]
pub mod vulkan_buffers;

#[cfg(feature = "opengl")]
#[macro_use]
extern crate glium;
#[cfg(feature = "opengl")]
pub mod opengl;
#[cfg(feature = "opengl")]
pub mod opengl_buffers;
