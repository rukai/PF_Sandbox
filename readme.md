# PF ENGINE

A platform fighter engine, with tightly integrated gameplay and character editor.

## Requirements:

*    Rust
*    libusb

### installing libusb on windows
You must use the GNU compatible rust version. (instead of MSVC)

Install [msys2](msys2.github.io).

Then in the msys2 terminal run:
`pacman -Syu mingw64/mingw-w64-x86_64-pkg-config`
`pacman -Syu mingw64/mingw-w64-x86_64-libusb`

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`
