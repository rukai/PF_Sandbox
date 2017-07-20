# PF Sandbox [![Build Status](https://travis-ci.org/rukai/PF_Sandbox.svg?branch=master)](https://travis-ci.org/rukai/PF_Sandbox) [![Build status](https://ci.appveyor.com/api/projects/status/89drle66lde9pq35?svg=true)](https://ci.appveyor.com/project/rukai/pf-sandbox)

A platform fighter sandbox featuring a character editor tightly integrated with gameplay.

Check out [this youtube video](https://www.youtube.com/watch?v=CTrwvg56VQs) demonstrating the project.  
[![Youtube Video](https://img.youtube.com/vi/CTrwvg56VQs/0.jpg)](https://www.youtube.com/watch?v=CTrwvg56VQs)

## Controller requirements

Controller support is currently hardcoded to the GC adapter for Wii U (Nintendo or Mayflash in Wii U mode)
Follow the steps for your OS, found under Installation at this [Dolphin Wiki page](https://wiki.dolphin-emu.org/index.php?title=How_to_use_the_Official_GameCube_Controller_Adapter_for_Wii_U_in_Dolphin)

## Setup on Windows

Install rust via https://www.rustup.rs/
Do a custom install and select nightly and GNU compatible rust version. (instead of MSVC)

Install [msys2](http://www.msys2.org/), following ALL of the instructions.

Then in the msys2 terminal run:
`pacman -Syu mingw64/mingw-w64-x86_64-pkg-config mingw64/mingw-w64-x86_64-libusb mingw-w64-x86_64-gcc`

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`

### Breakdown of crates -> msys2 package dependencies
#### Libusb:
*   mingw64/mingw-w64-x86_64-pkg-config
*   mingw64/mingw-w64-x86_64-libusb

#### zip->(flate->miniz-sys & bzip2) & rust-crypto
*   mingw-w64-x86_64-gcc

## Setup on Ubuntu

Install rust via https://www.rustup.rs/
Do a custom install and select nightly all other settings default.

sudo apt-get install libssl-dev libusb-1.0-0-dev cmake libvulkan-dev vulkan-utils

You will also need vulkan drivers:
*   Intel: sudo apt-get install mesa-vulkan-drivers
*   Nvida: No extra drivers required
*   AMD:   TODO

If it fails to launch, you may need to enable DRI3,
Create a file /etc/X11/xorg.conf.d/20-intel.conf containing:
```
Section "Device"
   Identifier  "Intel Graphics"
   Driver      "intel"
   Option      "DRI" "3"
EndSection
```

## Setup on Arch
deps: gcc, libusb, cmake

need vulkan drivers: vulkan-icd-loader
*   Intel: vulkan-intel
*   Nvida: No extra drivers required
*   AMD:   vulkan-radeon

## Compile and run

To run pf sandbox: run `cargo run` in the pf_sandbox directory.

## Setup CLI Tool

The tutorial/manual assumes that you have setup the binary for the CLI tool in your system path as the command `pf`.
To build the CLI tool run `cargo build` in the cli directory, the resulting binary is stored at `cli/target/debug/pf_client`.

However you can avoid fiddling with the system path by running `cargo run -- COMMAND` in the cli directory instead of `pf COMMAND`.

## Usage Documentation

*   [Get Started Tutorial](editor-tutorial.md)
*   [Full Reference Manual](manual.md)

## Goals/Features

*   Package system used to distribute complete games that run on PF Sandbox
    +   A package includes:
        -   Fighters
        -   Stages
        -   Rules - Set game mode and mechanics e.g. game length, stock count, l-canceling, ledge-hog mechanic
        -   A url specifying where to download updates
    +   Package data is serialized into multiple files stored in a folder, allowing individual characters/stages to be easily copied between packages
*   Powerful Fighter/Stage editor
    +   Make edits in the middle of a match
    +   Use the mouse to select elements for editing.
    +   Command line used for viewing/setting selected elements
    +   Keyboard shortcuts and click and drag where applicable
*   Replays that do not desync on character/mechanics/physics changes
*   Controller support including Native Wii U -> GC adapter Support
*   TAS Tools
*   Netplay
*   Minimalist but visually appealing graphics

## Non-Goals/Restrictions

*   Advanced features need not be beginner Friendly (e.g. editor/frame advance/replays/TAS)
*   Ability to recreate other platform fighters does not overrule other advantages (e.g. 2D hitboxes instead of 3D hitboxes)
*   Restricting character graphics to only hitboxes reduces scope for development of the project and development of packages

## CI Infrastructure

Note: There are currently no tests implemented yet.

We build and test on:

*   Rust nightly - Linux 64 bit (Travis)
*   Rust nightly gnu - Windows 64 bit (Appveyor)

All incoming pull requests are built and tested.

Every commit merged to master is built, tested and then an incrementing tag/release is created for it.

TODO: If the commit is tagged with Netplay, it is also released as the netplay build which is pinned at the top of releases.
