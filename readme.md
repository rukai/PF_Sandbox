# PF Sandbox [![Build Status](https://travis-ci.org/rukai/PF_Sandbox.svg?branch=master)](https://travis-ci.org/rukai/PF_Sandbox)

A platform fighter featuring a character editor tightly integrated with gameplay.

## Quick links

*   [pfsandbox.net](https://pfsandbox.net)
*   [Compile from source (Windows & Linux)](compiling.md)
*   [Discord](https://discord.gg/KyjBs4x)
*   [Infrastructure Repository](https://github.com/rukai/pf_sandbox_infra)

## OS/Controller requirements

*   Windows 10: Xbox controllers + native GC adapter
*   Other Windows: [Unsupported](https://gitlab.com/Arvamer/gilrs/commit/56bf4e2d04c972a73cb195afff2a9a8563f6aa34#note_58842780)
*   Linux: All controllers + native GC adapter
*   Mac OS: Unsupported

You cannot use a keyboard to play, you must use a controller.

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

## Restrictions/Non-Goals

*   Advanced features need not be beginner Friendly (e.g. editor/frame advance/replays/TAS)
*   Ability to recreate other platform fighters does not overrule other advantages (e.g. 2D hitboxes instead of 3D hitboxes)
*   Restricting character graphics to only hitboxes reduces scope for development of the project and development of packages

## CI Infrastructure

We build and test on:

*   Rust stable/nightly - Linux 64 bit (Travis)
*   Rust stable/nightly GNU - Windows 64 bit (Appveyor)

We build and test when:

*   All incoming pull requests are built and tested.
*   Every commit merged to master is built, tested and then an incrementing tag/release is created for it.
