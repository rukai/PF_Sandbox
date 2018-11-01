#![windows_subsystem = "windows"]

#![feature(drain_filter)]
#![feature(nll)]

             extern crate byteorder;
             extern crate chrono;
             extern crate enum_traits;
             extern crate getopts;
             extern crate gilrs_core;
             extern crate libusb;
             extern crate lyon;
             extern crate rand;
             extern crate serde;
             extern crate treeflection;
             extern crate winit;
             extern crate winit_input_helper;
             extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
#[macro_use] extern crate treeflection_derive;
#[macro_use] extern crate pf_sandbox_lib;

pub(crate) mod ai;
pub(crate) mod app;
pub(crate) mod camera;
pub(crate) mod cli;
pub(crate) mod collision;
pub(crate) mod game;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod menu;
pub(crate) mod particle;
pub(crate) mod player;
pub(crate) mod replays;
pub(crate) mod results;

#[cfg(feature = "vulkan")]
#[macro_use]
extern crate vulkano;
#[cfg(feature = "vulkan")]
extern crate vulkano_shaders;
#[cfg(feature = "vulkan")]
extern crate vulkano_win;
#[cfg(feature = "vulkan")]
extern crate vulkano_text;
#[cfg(feature = "vulkan")]
extern crate cgmath;

#[cfg(feature = "vulkan")]
pub(crate) mod vulkan;

use crate::app::run;
use crate::cli::cli;
use pf_sandbox_lib::config::Config;
use pf_sandbox_lib::logger;

fn main() {
    setup_panic_handler!();
    logger::init();
    let config = Config::load();
    run(cli(&config), config);
}
