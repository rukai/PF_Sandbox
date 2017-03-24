# PF Sandbox

A platform fighter sandbox featuring a character editor tightly integrated with gameplay.

## Requirements:

*    Rust
*    libusb

### Installing libusb on windows

You must use the GNU compatible rust version. (instead of MSVC)

Install [msys2](msys2.github.io).

Then in the msys2 terminal run:
`pacman -Syu mingw64/mingw-w64-x86_64-pkg-config`
`pacman -Syu mingw64/mingw-w64-x86_64-libusb`

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`

## Compile and run

The usual manner for rust programs: run `cargo run` in the src directory.

## Goals/Features

*   Package system used to distribute complete games that run on PF Sandbox
    +   A package includes:
        -   Fighters
        -   Stages
        -   Rules - Set game mode and mechanics e.g. game length, stock count, l-canceling, ledge-hog mechanic
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
