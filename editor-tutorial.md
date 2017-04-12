# Tutorial

## Editor

Following this tutorial is a great way to get started with PF Sandbox.
For a full reference of every operation check out the [documentation]().

First Pause a currently running game, with the spacebar key

Press `1` to select player's fighter.
Player 1 should now have a green outline.

Position your cursor where you wish to place a colbox (Collision Box) and press `F`.
Player 1 should now have an extra colbox

Now select that colbox with left click.
The colbox should be fully green.

Press the A key to move the selected colbox.
Move your mouse and left click to confirm the position.

## Command Line

A command looks like this:

`pf package.fighters[?].actions[?].frames[?].colboxes[0].size set 10`

The words: package, fighters, actions, frames are attributes these are used to specify what you want to interact with (e.g. all package.fighters means all the fighters in the loaded package)
The square brackets are indexes, they are used to specify which one you want to interract with (e.g. which fighter)
You can enter this example into your systems command line and hit enter to run it.

This will change the first colbox, because we used an index of 0, which ever one that is.
But thats generally not useful.

Now select a hitbox and run this command:
`pf package.fighters[?].actions[?].frames[?].colboxes[?].size set 10`
The colboxes index will be based on your colbox selection, due to the use of the context index '?'

Select a colbox and run this command:
`pf package.fighters[?].actions[?].frames[?].colboxes[?].role variant Hit
This changes the role of the colbox to be a hitbox.
Test it out in game.

Now that we have set the role to hit, we have access to more properties on the role.
`pf package.fighters[?].actions[?].frames[?].colboxes[?].role[0].bkb set 9001
We set the base knockback for the hitbox to 9001
Test this out in game!
