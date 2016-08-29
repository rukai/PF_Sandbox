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

Always backup any packages you are working on.
Due to the nature of the interface it is super easy to accidentally destroy work without realising.

To work through some examples check out the [editor-tutorial](editor-tutorial.md).

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
*   F9 - Dont display fighter
*   F10 - Display player camera area
*   F11 - Set all
*   F12 - Reset all

### Selection

By default making a selection will replace the previous selection.
To add to the previous selection hold the Shift key.

*   Left click - Select one hitbox
*   Right click - Select mutliple hitboxes

### Frame editing
*   V - Copy frame
*   B - Paste frame
*   N - Delete frame
*   M - Insert frame, copies from previous frame

### Hitbox editing

Most of these operations will apply to all selected hitboxes:

*   A - Move hitboxes, left click to confirm
*   S - Toggle pivot mode
*   D - Delete selected hitboxes
*   F - Insert hitbox meld linked to selected hitboxes, left click to confirm
    +   Shift: simple link
*   G - Resize hitboxes
*   Z - Meld link hitboxes
*   X - Simple link hitboxes
*   C - Unlink hitboxes

Linking collision boxes allows them to be pivoted in pivot mode.
Meld links combines collisionboxes into a single collisionbox.

Hitbox data can be set via a command:
*   Hitbox (damage, bkb, kbg, angle, clang)
*   Hurtbox (armor)

### Pivot mode

When there is one collisionbox selected, pressing `S` will enter pivot mode.

The selected collisionbox becomes the root collision box.
Any collisionbox can now be click and drag'ed around the root box.

Press `S` again to leave pivot mode.

## Command line

While PF ENGINE is running you can send commands to it via your systems command line.

The pf engine command line is very powerful, at the price of complexity.
We recommend you work through the Command Line section of the [Editor Tutorial](editor-tutorial.md) first, to get a feel for what commands are.
Then come back and learn the rules that commands follow and how to construct your own.

### Breakdown

Lets give a quick breakdown of an example command.
This command sets the weight of someFighter in the package myPackage to 1.2:

`pf packages.myPackage.fighters.someFighter.weight set 1.2`

*   pf          - the program name, tells your OS what command you want to run
*   packages    - attribute
*   myPackage   - attribute
*   fighters    - attribute
*   someFighter - attribute
*   weight      - attribute
*   set         - command
*   1.2         - value

We can see a command consists of: attributes then a command then values.

### Objects

Objects are unique entities within PF ENGINE.
They contain attributes which can be any of the following value types:
*   string  - some text
*   integer - a number
*   float   - a number with a decimal point
*   bool    - a true or false value
*   object  - another object

### Actions

Different objects support different actions:

All objects support the following actions:
*   <attribute> set <value> - change an attribute to the specified size
*   <attribute> get <depth> - display an attribute, the depth argument is optional and specifies how deeply nested object attributes should be shown.
*   <attribute> copy        - copy the specified attribute
*   <attribute> paste       - paste the copied attribute to the specified attribute (Must be the same type)

Attributes that are assigned some point in space can use the following
*   <attribute>.rotate <degrees> - rotate the object, around some central point, the specified number of degrees

### Object structure

Objects contain other objects creating a large tree:

*   Players
*   Debug
*   Packages
    +   Fighters
        -   <fighter name>
            *   action_defs
                +   <action index>
                    -   ActionFrame
                        *   colboxes
                            +   <colbox index>
                                -   CollisionBoxRole
                        *   colbox_links
                            +   <link index>
                        *   effects
    +   Stages
    +   Rules

### Object attributes

Show full detail of each object here:
Could probably script this.

### Context

The context of PF ENGINE (e.g. selected hitboxes) will automatically prefix the commands with the required context.
This means you can run the command:

`pf size set 50`

Instead of:

`pf packages <selected package> fighters <selected fighter> actions <current action> frames <current frameIndex> hitboxes <selected hitbox> size set 50`

### Stepping Back Context

TODO: How to handle clash of attribute names in context? Options include:
*   automatically choose the one that is nested deeper or shallower
*   syntax to force deepest or shallowest.
*   error without completing action

You can choose to step back the context any amount:

`pf frame.0.hitbox.2.size set 50`

Will run:

`pf packages.<selected package>.fighters.<selected fighter>.actions.<current action>.frames.<current frameIndex>.hitboxes.2 size set 50`

### Multiple Selections

If you have multiple selections, then the command will be run on every selection:
This means if you have selected hitboxes 2 and 4 and run:

`pf size set 50`

will run:

`pf package.<packagename>.fighter.<fightername>.action.<actionID or ActionName>.frame.<frameIndex>.hitbox.2 size set 50`
`pf package.<packagename>.fighter.<fightername>.action.<actionID or ActionName>.frame.<frameIndex>.hitbox.4 size set 50`
