# PF Sandbox Manual

Note: Everything labeled (TODO) is planned but may or may not be added in the future.

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

Packages are how unique games are made and shared in PF Sandbox.
A package contains:
*   Stages
*   Fighters
*   Game Rules

## Game Editing

Always backup any packages you are working on.
Due to the nature of the interface it is super easy to accidentally destroy work without realising.

To work through some examples check out the [editor-tutorial](editor-tutorial.md).

The pause screen is where all the editing action occurs.
Make sure you are on the pause screen to make use of the following tools.

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
*   F5 - Main stick vector (TODO)
*   F6 - CStick vector (TODO)
*   F7 - DI vector (TODO)
*   F8 - Display ECB and BPS
*   F9 - Dont display fighter
*   F10 - Display player camera area
*   F11 - Set all
*   F12 - Reset all

### Selection

By default making a selection will replace the previous selection.
To add to the previous selection hold the Shift key.

*   Left click - Select one colbox
*   Right click - Select mutliple colboxes

### Frame editing
*   V - Copy frame
*   B - Paste frame
*   N - Delete frame
*   M - Insert frame, copies from previous frame

### Hitbox editing

Most of these operations will apply to all selected colboxes:

*   A - Move colboxes, left click to confirm
*   S - Toggle pivot mode (TODO)
*   D - Delete selected colboxes
*   F - Insert colbox meld linked to selected colboxes, left click to confirm
    +   Shift: simple link
*   [ - Shrink selected colboxes
*   ] - Grow selected colboxes
*   Z - Meld link colboxes (TODO)
*   X - Simple link colboxes (TODO)
*   C - Unlink colboxes (TODO)

Linking collision boxes allows them to be pivoted in pivot mode.
Meld links combines collisionboxes into a single collisionbox.

### Pivot mode (TODO)

When there is one collisionbox selected, pressing `S` will enter pivot mode.

The selected collisionbox becomes the root collision box.
Any collisionbox can now be click and drag'ed around the root box.

Press `S` again to leave pivot mode.

### Stage Editing (TODO)

## Command line

While PF Sandbox is running you can send commands to it via your systems command line.

The pf engine command line is very powerful, at the price of complexity.
We recommend you work through the Command Line section of the [Editor Tutorial](editor-tutorial.md) first, to get a feel for what commands are.
Then come back and learn the rules that commands follow and how to construct your own.

### Breakdown

Lets give a quick breakdown of an example command.
This command sets the weight of someFighter in the package myPackage to 1.2:

`pf packages["myPackage"].fighters["someFighter"].weight.set 1.2` (TODO: Use this as example when named packages/fighters can be addressed by name)
`pf package.fighters[?].weight set 1.2`

*   pf          - the program name, tells your OS what command you want to run
*   package     - attribute
*   fighters    - attribute
*   ?           - context index
*   weight      - attribute
*   set         - command
*   1.2         - value

We can see a command consists of: attributes and indexes followed by a command then values.

### Objects

Objects are unique entities within PF Sandbox.
They contain attributes which can be any of the following value types:
*   string  - some text
*   integer - a number
*   float   - a number with a decimal point
*   bool    - a true or false value
*   object  - another object
*   list    - a list of objects
*   dict    - a dictionary of objects

Full [object reference](link_to_resource)

### Actions

Different objects support different actions:

All objects support the following actions:
*   <attribute> help <value> - view an objects type, its attributes and its commands
*   <attribute> set <value>  - change an attribute to the specified size
*   <attribute> get <depth>  - display an attribute, the depth argument is optional and specifies how deeply nested object attributes should be shown.
*   <attribute> copy         - copy the specified attribute
*   <attribute> paste        - paste the copied attribute to the specified attribute (Must be the same type)

Attributes that are assigned some point in space can use the following (TODO)
*   <attribute>.rotate <degrees> - rotate the object, around some central point, the specified number of degrees

### Object structure

Objects contain other objects creating a large tree:

[TODO: Replace with a nice diagram that doesnt look like death]
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

### Getter

Get objects using dot notation

`pf package.fighters[?].actions[?].frames[?].hitboxes[?].size set 50`

### Indexing

The indexer is powerful:

*   `packages["M"]`            dictionaries can be accessed via strings (TODO)
*   `actions[0]`               select package 0
*   `actions[0, 1-5]`          select packages 0 and packages between 1 and 5 inclusive (TODO)
*   `actions[2-4].fighters[*]` select all fighters in packages 2, 3 and 4 (TODO)
*   `actions[*]`               select all packages (TODO)
*   `actions[?]`               select based on [context](link_to_context_section)
*   `actions[?+1]`             TODO
*   `actions[2, ?-1]`          TODO

### Context

There are many contexts available that allow you to quickly hook into the object you want to modify.
Setting some hitboxes to size 50 can be done the primitive way.
Objects are chained together with '.' and indexed by [] like this:

`pf package.fighters[4].actions[0].frames[0].colboxes[1,5-7].size set 50` (TODO)

However how are you supposed to know all of these indexes? o.0
Instead you can let PF engine use context to know what you want to modify.
Select the hitboxes you want in game then run:

`pf package.fighters[?].actions[?].frames[?].colboxes[?].size.set 50`

### Aliases

Its super long to type in all this junk just to get to some hitboxes
Take advantage of your shell and add this to your .bashrc
alias pf-hitboxes="`pf package.fighters[?].actions[?].frames[?].colboxes[?]`

Eh, this wont work exactly because the alias will only activate if there is a space between it and the argument. (TODO, will probably have to build internal shortcuts or something)
