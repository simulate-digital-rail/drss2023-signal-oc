# DRSS 2023 - EULYNX Live

## Build Instructions

To build for RevPi run:

```
cargo build --release
```

You also need to have the `arm-linux-gnueabihf-gcc` compiler installed and added to your `$PATH`.
For Windows, you can download it from the [ARM developer page](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads)
(the most recent version should be [this one](https://developer.arm.com/-/media/Files/downloads/gnu/12.2.rel1/binrel/arm-gnu-toolchain-12.2.rel1-mingw-w64-i686-arm-none-linux-gnueabihf.zip?rev=594a0e67053b41a69bef8ec31614ae63&hash=2D1826C238F9ECE7A86DB9FE99AE9E25E137D59F)). 
On Linux, it may be installable through your package manager (e.g. `sudo apt install gcc-arm-linux-gnueabihf` under Debian). If not,
find the Linux version of the toolchain on the ARM developer page.

You can also run the code locally on your machine.
To build for Windows run:

```
cargo build --target x86_64-pc-windows-msvc
```

To build for Linux run:

```
cargo build --target x86_64-unknown-linux-gnu
```

Alternatively, use `cargo run` instead of `cargo build` in order to
run the project directly.