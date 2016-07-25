# PF ENGINE Manual

## WiiU adapter setup

Follow the steps for your OS, found under Installation at this [Dolphin Wiki page](https://wiki.dolphin-emu.org/index.php?title=How_to_use_the_Official_GameCube_Controller_Adapter_for_Wii_U_in_Dolphin)
There is no need to perform the steps listed under Dolphin Setup.

## Gameplay

Select characters, select stages and FIGHT!

These player outlines are used to signify which team they belong to:
*   Orange
*   Blue

### Camera

Middle click to pan the camera.
Scroll to zoom the camera.
Press Backspace to return control of the camera to the game.

## Packages

Packages are how unique games are made and shared in PF ENGINE.
A package contains:
*   Stages
*   Fighters
*   Game Rules

## Game Editing

To work through some examples check out the [editor-tutorial]().

The pause screen is where all the editing action occurs.
Make sure your on the pause screen to make use of the following tools.

### Game Flow

Use the following keys to alter the flow of the game
*   Spacebar - step game
*   H - Rewind
*   J - Step rewind
*   K - Step replay
*   L - Replay

### Editor Selector

First you must select what you wish to edit:

*   ` - World
*   1 - Player 1's fighter
*   2 - Player 2's fighter
*   3 - Player 3's fighter
*   4 - Player 4's fighter
*   1 + Shift - Player 1
*   2 + Shift - Player 2
*   3 + Shift - Player 3
*   4 + Shift - Player 4

### Debug Displays

Debug output is displayed to the terminal every time a frame is changed or modified.
Use the following keys to toggle debug displays:

*   F1 - Player physics  (terminal only)
*   F2 - Input (terminal only)
*   F2 + Shift - Input difference (terminal only)
*   F3 - Current action (terminal only)
*   F4 - Frame (terminal only)
*   F5 - Main stick vector
*   F6 - CStick vector
*   F7 - DI vector
*   F8 - Display ECB and BPS
*   F9 -
*   F10 -
*   F11 - Set all
*   F12 - Reset all

### Selection

By default making a selection will replace the previous selection.
To add to the previous selection hold the Shift key.

*   Left click - Select one hitbox
*   Right click - Select mutliple hitboxes

### Frame editing

*   N - Delete frame
*   M - Insert frame, copies from previous frame

### Hitbox editing

Most of these operations will apply to all selected hitboxes:

*   A - move hitboxes, left click to confirm
*   S - resize hitboxes
*   D - delete selected hitboxes
*   F - Insert hitbox, left click to confirm
*   Z - Meld link hitboxes
*   X - Simple link hitboxes
*   C - Unlink hitboxes

Melding hitboxes combines them into a single hitbox.
Pivot hitboxes act as if they are melded but remain as seperate hitboxes.

Hitbox data can be set via a command:
*   Hitbox (damage, bkb, kbg, angle, clang)
*   Hurtbox (armor)
