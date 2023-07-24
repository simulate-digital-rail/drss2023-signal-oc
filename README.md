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

## gRPC example
### Starting the gRPC bridge(s) on x86_64/amd64 systems
For these systems, we provide a Docker Compose File that allows you to easily set up the bridges. Therefore, it is as easy as running `docker-compose up -d`.

### Starting the gRPC bridge(s) on other systems (including RevPi)
Here, we cannot provide working Docker images. Therefore, you have to manually compile the gRPC bridge (on the RevPi, we already built a binary for you).
For this, follows these steps (on RevPi, skip the first two):

* run `git submodule update --init` to download the RaSTA sources and `cd rasta-protocol`
* build the gRPC bridge binary by running the commands that are given in the steps "Install CUnit", "Install gRPC", "Configure CMake without extensions" (without the `-DENABLE_RASTA_TLS=ON`) and "Build without extensions" of [this CI config](https://github.com/eulynx-live/rasta-protocol/blob/main/.github/workflows/ci.yml)
    * **Note**: in this step, you might need to fix some small compiler errors, especially when using 32-bit OSes like Raspbian or when using `clang` compiler
* the binary now is in `rasta-protocol/build`, `cd` into this directory
* start the gRPC bridge (as a Rasta server / gRPC client) on the interlocking side with the command `./rasta_grpc_bridge_udp ../../config/rasta_interlocking.cfg 0.0.0.0:4242 127.0.0.1 9998 127.0.0.1 9999 97 96 127.0.0.1:50001` run in the build folder of the rasta-protocol project. First parameter is the Rasta config file, second will be ignored, 3-6 are the transport channels of the Rasta client, 7-8 are the Rasta IDs of server and client, 9 is the address of the gRPC server to connect to.
* start the gRPC bridge (as a Rasta client / gRPC server) on the OC side with the command `./rasta_grpc_bridge_udp ../../config/rasta_oc.cfg 0.0.0.0:50002 127.0.0.1 8888 127.0.0.1 8889 96 97` run in the build folder of the rasta-protocol project. First parameter is the Rasta config file, second is the socket the gRPC server should listen on, 3-6 are the transport channels of the Rasta server, 7-8 are the Rasta IDs of client and server.

### Starting the gRPC example
Run the following steps:

* start the interlocking software (`grpc_sender`) with the command `cargo run --bin grpc_sender 0.0.0.0 50001` (gRPC server will listen on port 50001)
* refer to the respective subsection, depending on your processor architecture, to  start the gRPC bridges for OC and interlocking.
* start the OC software (`grpc_main`) with the command `cargo run --bin grpc_main 127.0.0.1 50002` (gRPC client will connect to the server on port 50002)


## rasta-rs example 
* start the OC software (`main`) with the command `cargo run --bin receiver`
* start the interlocking software (`sender`) with the command `cargo run --bin sender` 

The sender can send the main aspect Ks1 or Ks2.
Writing to the console and confirm with enter.