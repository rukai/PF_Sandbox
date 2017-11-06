# PF Sandbox Manual

Note: Everything labeled (TODO) is planned but may or may not be added in the future.

## WiiU adapter setup

Follow the steps for your OS, found under Installation at this [Dolphin Wiki page](https://wiki.dolphin-emu.org/index.php?title=How_to_use_the_Official_GameCube_Controller_Adapter_for_Wii_U_in_Dolphin)
There is no need to perform the steps listed under Dolphin Setup.

## Gameplay

Use `a` on your GC controller to: select package, select local, select characters, select stages and FIGHT!
Use `b` on your GC controller to go to previous menus (keep pressing to reach package select screen)

The player outlines are used to signify which team they belong to, you can change this on the CSS.

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
*   A source URL to retrieve updates from

You can manually download packages and place them in the packages folder:
*   Linux: ~/.local/share/PF_Sandbox/packages
*   Windows: C:\Users\Username\Appdata\Local\PF_Sandbox\packages

You can safely copy fighter and stage files between packages.

You can use the `:open_package $package_folder_name` command on the menu to open the package stored at the specified folder name.
If the package does not exist at that folder a new package is created there.

## Editor

Always backup any packages you are working on.
Due to the nature of the interface it is super easy to accidentally destroy work without realising.

To work through some examples check out the [editor-tutorial](editor_tutorial.md).

The pause screen is where all the editing action occurs.
Make sure you are on the pause screen to make use of the following tools.

### Game Flow

Use the following keys to alter the flow of the game
*   Spacebar - step game
*   H        - Rewind
*   J        - Step rewind
*   K        - Step replay
*   L        - Replay

### Editor Selector

Before editing anything you must first select what you wish to edit:

*   0         - Stage
*   1         - Player 1's fighter
*   2         - Player 2's fighter
*   3         - Player 3's fighter
*   4         - Player 4's fighter
*   1 + Shift - Player 1
*   2 + Shift - Player 2
*   3 + Shift - Player 3
*   4 + Shift - Player 4

### Element Selection

Sometimes you need to make ANOTHER selection with your mouse.
In different editor modes you can select different elements to modify them.

*   Left click           - Select one element
*   Right click and drag - Select multiple elements

#### Modifiers

*   Default   - Delete the previous selection
*   Shift key - Add to the previous selection
*   Alt key   - Remove from previous selection

### Debug Displays

While in the relevant editor mode (keys 0-9) you can toggle various debug displays.
Textual debug output is also written to stdout every time a frame is changed or modified.
Use F1-F12 to toggle them.

## Fighter Editor (1-9)

### Debug Displays

*   F1 - Player physics
*   F2 - Input
*   F2 + Shift - Input difference
*   F3 - Current action
*   F4 - Timers and Counters
*   F5 - Control stick (white) + C Stick vector (yellow)
*   F6 - pre DI vector (red) + post DI vector (green)
*   F7 - hitbox BKB vector (white) + hitbox KBG (blue)
*   F8 - Display ECB and BPS
*   F9 - Dont display fighter
*   F10 - Display player camera area
*   F11 - Set all
*   F12 - Reset all

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

## Stage Editor (0)

### Debug Displays

*   F1 - Display blast zone boundary
*   F2 - Display camera boundary
*   F3 - Display/edit spawn points
*   F4 - Display/edit respawn points
*   F11 - Set all
*   F12 - Reset all

### Element Editing

The following operations apply to all selected platforms and re/spawn points

*   A - Move element
*   D - Delete element
*   F - Place platform
*   Z - Place spawn point
*   X - Place respawn point
*   C - Connect/disconnect surfaces

## Command line

Press '~' to open the command line in PF Sandbox.
Alternatively you can use the pf_cli binary to send commands via your OS's terminal to PF Sandbox.
Note that when using pf_cli you will need to escape characters that have special meaning to your shell. e.g. quotes

The pf engine command line is very powerful, at the price of complexity.
We recommend you work through the Command Line section of the [Editor Tutorial](editor-tutorial.md) if you have not yet worked with the PF Sandbox command line tool.
You can then come back here if you ever need a reference.

### Breakdown

Lets give a quick breakdown of an example command.
This command sets the weight of someFighter in the package myPackage to 1.2:

`pf package.fighters[base_fighter.json].weight:set 1.2`

*   pf                  - the program name, tells your OS what command you want to run
*   package             - property
*   fighters            - property
*   [base_fighter.json] - key
*   weight              - property
*   set                 - command
*   1.2                 - value

A command consists of:
1.  properties, indexes and keys to select an object.
2.  A command to do something to the selected object.
3.  Optional values to define the command.

### Actions

Different objects support different commands:

All objects support the following commands:
*   help <value> - view an objects type, its attributes and its commands
*   set <value>  - change an attribute to the specified size
*   get <depth>  - display an attribute, the depth argument is optional and specifies how deeply nested object attributes should be shown.
*   copy         - copy the specified attribute
*   paste        - paste the copied attribute to the specified attribute (Must be the same type)

Properties that are assigned some point in space can use the following (TODO)
*   rotate <degrees> - rotate the object, around some central point, the specified number of degrees

### Object structure

Objects contain other objects creating a large tree:

[TODO: Replace with a nice diagram that doesn't look terrible and is hopefully autogenerated]
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

Get object properties with dot notation. 

`object.property`

### Indexing

Sometimes an objects properties require an index or key to access them.

*   `property["key_string"]` select by key
*   `property[0]`            select by index
*   `property[?]`            select based on [context](link_to_context_section)
*   `property[*]`            select all

TODO: Planned fancy index accessors
*   `property[0, 3]` select 0 and 3 (TODO)
*   `property[1-5]`  select 0 and between 1 and 5 inclusive (TODO)
*   `property[?+1]`  one more then context (TODO)

### Context

There are many contexts available that allow you to quickly access the object you want to modify.
Setting a colbox to radius 10 can be done the primitive way.
Objects are chained together with '.' and indexed by [] like this:

`package.fighters["fighter.json"].actions[0].frames[0].colboxes[2].radius:set 10`

However how are you supposed to know all of these indexes? o.0
Instead you can let PF engine use context to know what you want to modify.
Select the hitboxes you want in game then run:

`pf package.fighters[?].actions[?].frames[?].colboxes[?].radius:set 10`

## How to Publish a Package

1.  Add a source property to your package_meta
2.  Run `package:publish`
3.  Then grab the files at ~/.local/share/PF_Sandbox/publish/
4.  Upload them to your webserver at the url you specified in the source property

Note:
If PF Sandbox encounters any errors while updating a package, it will "silently" fail and continue loading the package as normal.
This is because we dont want to prevent users playing local games due to network issues.
However to troubleshoot issues with PF Sandbox downloading/updating your package you can check stdout for errors.
