# Setup for Windows

Install rust via https://www.rustup.rs/
Do a custom install and select nightly and GNU compatible rust version. (instead of MSVC)

Install [msys2](http://www.msys2.org/), following ALL of the instructions.

Then in the msys2 terminal run:
`pacman -Syu mingw64/mingw-w64-x86_64-pkg-config mingw64/mingw-w64-x86_64-libusb mingw-w64-x86_64-gcc mingw-w64-x86_64-gtk3`

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`

## Setup GDB on Windows (Optional)

TODO

# Setup for Ubuntu

Install rust via https://www.rustup.rs/
Do a custom install and select nightly all other settings default.

sudo apt-get install libssl-dev libusb-1.0-0-dev pkg-config cmake libvulkan-dev vulkan-utils libudev-dev

You will also need vulkan drivers:
*   Intel: sudo apt-get install mesa-vulkan-drivers
*   Nvida: No extra drivers required
*   AMD:   TODO

If it fails to launch, you may need to enable DRI3,
Create a file /etc/X11/xorg.conf.d/20-intel.conf containing:
```
Section "Device"
   Identifier  "Intel Graphics"
   Driver      "intel"
   Option      "DRI" "3"
EndSection
```

# Setup for Arch

sudo pacman -Syu gcc libusb cmake

need vulkan drivers: vulkan-icd-loader
*   Intel: vulkan-intel
*   Nvida: No extra drivers required
*   AMD:   vulkan-radeon

# Compile and run PF Sandbox

run `cargo run --release` in the pf_sandbox directory.

# Compile and run PF TAS

run `cargo run --release` in the pf_tas directory.

# Compile and run PF Controller Mapper

run `cargo run --release` in the map_controllers directory.

# Setup PF CLI
To build the CLI tool run `cargo build` in the pf_cli directory, the resulting binary is stored at `target/debug/pf_cli`.
Copy `pf_cli` to somewhere in your PATH and rename it to `pf`.
