./rasta-protocol/build/rasta_grpc_bridge_udp config/rasta_oc.cfg \
0.0.0.0:50002 127.0.0.1 8888 127.0.0.1 8889 96 97 &
cargo run --bin receiver 127.0.0.1 50002 config/pin_config.toml 
