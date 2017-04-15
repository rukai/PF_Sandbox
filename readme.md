# PF Sandbox

A platform fighter sandbox featuring a character editor tightly integrated with gameplay.

## Hardware Requirements:
*   GC adapter for wii-u (nintendo or mayflash in wii-u mode)
*   Follow the steps for your OS, found under Installation at this [Dolphin Wiki page](https://wiki.dolphin-emu.org/index.php?title=How_to_use_the_Official_GameCube_Controller_Adapter_for_Wii_U_in_Dolphin)
*   regular controllers are planned but not ready yet

## setup on windows

You must use the latest nightly GNU compatible rust version. (instead of MSVC)

Install [msys2](http://www.msys2.org/).

Then in the msys2 terminal run:
`pacman -Syu mingw64/mingw-w64-x86_64-pkg-config`
`pacman -Syu mingw64/mingw-w64-x86_64-libusb`
`pacman -Syu mingw-w64-x86_64-openssl` (TODO: verify)
TODO: wont these work on the same line?

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`

## setup on ubuntu

Install rust via https://www.rustup.rs/
Do a custom install and select nightly all other settings default.

sudo apt-get install libssl-dev libusb-1.0-0-dev cmake libvulkan-dev vulkan-utils

You will also need vulkan drivers:
*   Intel: sudo apt-get install  mesa-vulkan-drivers
*   Nvida: TODO
*   AMD:   TODO

You will need to enable DRI3:

Create a file /etc/X11/xorg.conf.d/20-intel.conf containing:
Section "Device"
   Identifier  "Intel Graphics"
   Driver      "intel"
   Option      "DRI" "3"
EndSection

## Compile and run

To run pf sandbox: run `cargo run` in the pf_sandbox directory.

To run the CLI for issuing editor commands: run `cargo run` in the cli directory.

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
