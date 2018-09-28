# Setup for Windows

Setup for windows is a bit complicated because we use windows-msvc toolchain but crosscompile to the windows-gnu target.
We do this because we need windows-gnu for libusb, but windows-msvc seems better supported.

## Workaround
Currently you will need to also workaround https://github.com/rust-lang/rust/issues/49078

1.  download and extract https://s3-us-west-1.amazonaws.com/rust-lang-ci2/rust-ci-mirror/x86_64-6.3.0-release-posix-seh-rt_v5-rev2.7z
2.  add the absolute path to mingw64\bin to your PATH environment variable. (This path needs to be before the msys2 path)

## Regular steps
Install rust via https://www.rustup.rs/ using the default settings. If you already have rustup use the msvc toolchain.
Install [Build Tools for Visual Studio 2017](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2017)
Run the command `rustup target install x86_64-pc-windows-gnu`

Install [msys2](http://www.msys2.org/), following ALL of the instructions.

Then in the msys2 terminal run:
 `pacman --noconfirm -Syu mingw64/mingw-w64-x86_64-pkg-config mingw64/mingw-w64-x86_64-libusb mingw-w64-x86_64-gcc mingw-w64-x86_64-gtk3 mingw-w64-x86_64-cmake mingw-w64-x86_64-make`

Add the msys2 mingw64 binary path to the PATH environment variable.
In my case this was `C:\msys64\mingw64\bin`

## Setup GDB on Windows (Optional)

TODO

# Setup for Ubuntu

Install rust via https://www.rustup.rs/ (Use the default settings)

```
sudo apt-get install build-essential libssl-dev libusb-1.0-0-dev pkg-config cmake libvulkan-dev vulkan-utils libudev-dev
```

Need to also install one of the following packages depending on your graphics card:
*   Intel: sudo apt-get install mesa-vulkan-drivers
*   Nvidia: No extra drivers required
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

```
sudo pacman -Syu rustup gcc make python libusb cmake vulkan-icd-loader
```

Need to also install one of the following packages depending on your graphics card:
*   Intel: vulkan-intel
*   Nvidia: No extra drivers required
*   AMD:   vulkan-radeon

# Compile and run PF Sandbox

In the pf_sandbox directory run:
*   for windows: `$ENV:PKG_CONFIG_ALLOW_CROSS=1; cargo run --release --target x86_64-pc-windows-gnu`
*   for linux: `cargo run --release`

# Compile and run PF TAS

In the pf_tas directory run:
*   for windows: `$ENV:PKG_CONFIG_ALLOW_CROSS=1; cargo run --release --target x86_64-pc-windows-gnu`
*   for linux: `cargo run --release`

# Compile and run PF Controller Mapper

In the map_controllers directory run:
*   for windows: `$ENV:PKG_CONFIG_ALLOW_CROSS=1; cargo run --release --target x86_64-pc-windows-gnu`
*   for linux: `cargo run --release`

# Setup PF CLI
To build the CLI tool run `cargo build` in the pf_cli directory, the resulting binary is stored at `target/debug/pf_cli`.
Copy `pf_cli` to somewhere in your PATH and rename it to `pf`.
