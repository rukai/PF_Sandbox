# PF TAS

!!! Warning: Still uses vulkano 0.3.0 which will crash linux with a nvidia card, when resizing the window !!!

A keyboard driven GUI tool for sending TAS inputs to PF Sandbox.

## Keyboard mapping

### Inputs

####  Modifiers
*   Enter a number with 1234567890-+ before selecting an analog element (stick or trigger) to set the number as the value
*   Hold shift to keep pressed until toggled off

#### Keyboard -> GC

*   a          -> a
*   s          -> b
*   d          -> x
*   f          -> y

*   g          -> start
*   h          -> z

*   j          -> left button
*   k          -> left trigger
*   l          -> right trigger
*   ;          -> right button

*   y          -> stick horizontal
*   u          -> stick vertical
*   i          -> c-stick horizontal
*   o          -> c-stick vertical

*   arrow keys -> DPAD

### Frame Advance

Enter a number with 1234567890 before some of these keys to change the number of frames

*   Enter - Play/Pause toggle
*   Space - step number of frames
*   Z     - rewind number of frames
*   X     - replay number of frames
*   C     - rewind
*   V     - replay

### Config

*   F1-F12 Select a controller
*   }/{    Add/remove controller

*   Q - Toggle between display all controllers / display selected controller
*   W - Toggle stretch display / force aspect ratio
*   E - Toggle display float values / byte values for sticks and triggers
*   R - Toggle touch typing mode and 1-1 keybindings
