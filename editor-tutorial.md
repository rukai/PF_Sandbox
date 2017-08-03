# Editor Tutorial

Following this tutorial is a great way to get started with editing fighters in PF Sandbox.
For a full reference of every operation check out the [documentation]().

## Create a new package

It is important to create your own package to put your fighters in.
If you instead modify the example package, when an update is released for it, your modifications will be lost!

TODO:
Add a way to create a new package via treeflection. https://github.com/rukai/PF_Sandbox/issues/32

For Now:
Create a new package by running from your command line `pf_sandbox new_package_name`
After the game starts pause it. (spacebar)

## Basics

It will look like there is a stage with no fighters.
However the fighters are there, they just have no collision boxes to make them visible.
Start off by ensuring your controller is in port 1.
Press `1` to select player 1.
Now press F8 to toggle on ECB (Environment Collision Box) display
Now we can see player 1's location as the newly appeared ECB.

Repeat this for all players.
1.  Press 1
2.  Press F8
3.  Press 2
4.  Press F8
3.  Press 3
4.  Press F8
3.  Press 4
4.  Press F8

Unpause and move your fighter around

## Player states

Pause the game, select your player (1) before continuing.
Press F3, to enable player state debug information.
Frame advance with spacebar.
You can now see debug info in the terminal you launched PF Sandbox from. (TODO: in game display)

Hold X on your controller and press spacebar on your keyboard.
Your action will go from Idle -> JumpSquat.
Press spacebar again your action will go from JumpSquat -> JumpF.
Press spacebar again and again until your fighter lands.

## Playing with colboxes

Ensure your fighter is in its Idle state again.
Position your cursor, near your fighter (the green ECB), and press `F`.
Player 1 should now have a colbox (Collision Box).

Press F again somewhere else and notice that your new colbox is joined to the previous one.

This time deselect your new colbox by left clicking empty space.
Then press F again and notice your new colbox is not joined.

Now select any colbox with left click.
The colbox should be fully green.
Use the square bracket keys, [ and ], to resize the selected colbox.

Hold shift and left-click to select multiple hitboxes.
Press the A key.
Now you can move the the selected colboxes by moving your mouse.
Left click to confirm the new position.

Click and drag, with right click to select multiple colboxes.
Select all your colboxes and press D to delete them all.

Play around with colboxes to your hearts content, then test out your creation in game.

## Copying frames

At the conclusion of your testing you would have surely noticed that your beloved colboxes can only be seen while the player is standing still.
Lets fix that.

You could manually recreate your design on every action the fighter has, but thats a lot of wasted effort.
Instead lets copy the colboxes in the fighters idle state.
To all his other states as a base to work on.

Ensure the fighter is in his idle state, the game is paused and your player is selected.
Press V to copy the current frame.
Press X on your controller and space on your keyboard.
Now your player is in his jumpsquat frames.
Press B on your keyboard to paste the copied frame here.

Repeat this process for all actions you can think of.
Test out your fighter again, you can even disable the ECB (F3) if you want.

## Multi-frame actions

Return your player to the idle state, pause the game, and select your player.
Notice how the player state debug information says `frame: 0/0`.
This means the action Idle has only 1 frame and we are currently on that one frame.
We start counting at 0 here so that it fits in with other systems. (TODO: Although if Lua is used as the scripting language then it should really be 1-indexed)

Lets add another frame to this action by pressing M.
We can now see we are on frame 1 of 1.
Make some changes to the colboxes on this frame to differentiate it from frame 0.

Press M again and now we are on frame 2 of 2.
Once again make some changes to the colboxes.

Press space to advance through our 3-frame animation cycle.
We can see the debug info go `frame 0/2`, `frame 1/2`, `frame 2/2` and then restart at `frame 0/2`.

For some actions this will affect game logic.
Try adding 30 frames to your fighters jumpsquat action.
Notice how each frame needs to complete before the fighter will lift off the ground.

Restore your jumpsquat frames to something sensible by pressing N to delete frames, while in the jumpsquat action.

## Camera

As you are editing your fighter you will want to manually control the camera to better see your fighter.
Zoom in and out with the scroll wheel on your mouse.
Reposition the camera with middle click.
To give control of the camera back to PF Sandbox press Backspace.

## Command Line

Press '~' to open the command line in PF Sandbox.

A command looks like this:

`package:help`

We use the help command to tell us what an object is capable of and what other objects it contains.
By looking under the Accessors section, we can see that the package contains fighters, stages, meta, and rules.
Lets further investigate fighters.

`package.fighters:help`

## Menu/Game difference

While in game commands are run on the game object. (which contains the package)
While in menu commands are run on the package object.
So commands that work in game wont necessarily work in the menu and vice versa.

This is awkward to use so in the future a package holder object will be added so commands on the package succeed wherever they are run.

## Save/Reload

Run the command: `package:save`
Wait for the 'Save completed successfully' message to appear.
Close PF Sandbox and reopen it to verify that your changes are still there.

Make a change to your fighter and then run: `package:reload`
Verify that your fighter is the same as when you last ran `package:save`

## Keys & Context

This is a keyed context vector of fighters, that means it contains multiple fighters. (accessible by both a key, an index and context)

We can access a specific fighter via its filename, assuming you have a fighter with filename base_fighter.json then we can run:

`package.fighters["base_fighter.json"]:help`

But thats generally not useful. (We dont want to have to check the filename of the fighter we are editing.)
Instead we make use of the context system.
By using `?` as our key, we tell PF Sandbox to automatically choose the fighter to access via in game context.
In this case it uses the fighter used by the player we have selected with 1234.

`package.fighters[?]:help`

If this doesnt display the fighter help, ensure you are in a game, pf sandbox is paused (spacebar) and you have selected a player (1)

## Fighter data

The fighter help text shows a lot of interesting fighter properties that I am sure you are itching to play with.

This will get the number of aerial jumps the fighter can do.

`package.fighters[?].air_jumps:get`

Run this command and try out the changes.

`package.fighters[?].air_jumps:set 99`

Try fiddling with this and other values you can find with the help command.

## Descending the tree

Individual colboxes can be accessed and manipulated, however they are stored in an object in an object in an object in an object.
To find them we are going to do some exploration with the help command.
We see that the fighter contains an `actions` property which is a context vec.
(once again this object contains multiple actions, however they are only accessible by index and context.)

`package.fighters[?].actions:help`

We can use an index to access a numbered element of the vector

`package.fighters[?].actions[0]:help`

However once again this is rarely useful so we stick to using context.
This way we access the current action the selected player is in.

`package.fighters[?].actions[?]:help`

We can see two properties: iasa and frames.

`package.fighters[?].actions[?].frames[?]:help`

We can see the colboxes property among numerous other properties.

`package.fighters[?].actions[?].frames[?].colboxes:help`

Nice, now we know how to access colboxes, and (hopefully) better understand how frame data is structured.

## Colbox resize

Now select a hitbox by clicking on it in pf_sandbox and run this command:

`package.fighters[?].actions[?].frames[?].colboxes[?].radius:set 10`

## Variants & Hitboxes

Select a colbox and run this command:

`package.fighters[?].actions[?].frames[?].colboxes[?].role:variant Hit

This changes the role of the colbox to be a hitbox.
Test it out in game.

Now that we have set the role to hit, we have access to more properties on the role.

`package.fighters[?].actions[?].frames[?].colboxes[?].role[0].bkb:set 9001`

We set the base knockback for the hitbox to 9001
Test this out in game!
