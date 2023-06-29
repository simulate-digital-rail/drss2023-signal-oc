# DRSS 2023 - EULYNX Live

## Build Instructions

To build for RevPi, you need to have the `armv7-unknown-linux-gnueabihf` Rust target installed. Install it with:

```
rustup target add armv7-unknown-linux-gnueabihf
```

Then, to build the application run:

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

## Known issues

When cross compiling from Ubuntu, there may be a glibc version mismatch with the Revolution Pi. To work around it, you can compile this project directly on the Revolution Pi.

## Starting the gRPC example
Run the following steps:

* start the OC software (`grpc_main`) with the command `cargo run --bin grpc_main 0.0.0.0 50001` (gRPC server will listen on port 50001)
* run `git submodule update --init` to download the RaSTA sources (only if not on x86_64)
* run `docker-compose up -d` to start the gRPC bridges for both OC and interlocking (Note: On x86_64, to skip building the image, you can replace the `build: ..` lines for both services by `image: ghcr.io/eulynx-live/rasta-protocol/rasta_grpc_bridge_udp:main`)
* start the interlocking software (`grpc_sender`) with the command `cargo run --bin grpc_sender 127.0.0.1 50002` (gRPC client will connect to the server on port 50002)

Note: For the real OC software, these steps should be automated using Docker, Bash scripts or the like.

## Starting the rasta-rs example 
* start the OC software (`main`) with the command `cargo run --bin receiver`
* start the interlocking software (`sender`) with the command `cargo run --bin sender` 

The sender can send the main aspect Ks1 or Ks2.
Writing to the console and confirm with enter.