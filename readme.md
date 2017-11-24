# PF Sandbox [![Build Status](https://travis-ci.org/rukai/PF_Sandbox.svg?branch=master)](https://travis-ci.org/rukai/PF_Sandbox) [![Build status](https://ci.appveyor.com/api/projects/status/89drle66lde9pq35?svg=true)](https://ci.appveyor.com/project/rukai/pf-sandbox)

A platform fighter sandbox featuring a character editor tightly integrated with gameplay.

## Quick links

*   [Download for Windows](https://github.com/rukai/PF_Sandbox/releases/latest) (Run pf_sandbox.exe)
*   [Compile from source (Windows & Linux)](documentation/compiling.md)
*   [Youtube introduction](https://www.youtube.com/watch?v=CTrwvg56VQs) to the project.
*   [Editor Tutorial](documentation/editor_tutorial.md)
*   [PF Sandbox Manual](documentation/manual.md)
*   [TAS Manual](documentation/pf_tas.md)
*   [Discord](https://discord.gg/KyjBs4x)
*   [Infrastructure Repository](https://github.com/rukai/pf_sandbox_infra)

[![Youtube Video](https://img.youtube.com/vi/CTrwvg56VQs/0.jpg)](https://www.youtube.com/watch?v=CTrwvg56VQs)

## Controller requirements

Controller support is currently hardcoded to the GC adapter for Wii U (Nintendo or Mayflash in Wii U mode)
Follow the steps for your OS, found under Installation at this [Dolphin Wiki page](https://wiki.dolphin-emu.org/index.php?title=How_to_use_the_Official_GameCube_Controller_Adapter_for_Wii_U_in_Dolphin)

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

*   Rust nightly - Linux 64 bit (Travis)
*   Rust nightly gnu - Windows 64 bit (Appveyor)

We build and test when:

*   All incoming pull requests are built and tested.
*   Every commit merged to master is built, tested and then an incrementing tag/release is created for it.

TODO:
Github releases is fairly limited, a custom solution might be needed so that we can:
*   combine linux and windows builds into a single release (tried and failed to achieve this with Github releases)
*   Tag a commit as the current netplay build and then pin that build at the top. (no such functionality in Github releases)
