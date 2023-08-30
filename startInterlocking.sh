trap 'kill $INTERID; exit' INT
./rasta-protocol/build/rasta_grpc_bridge_udp config/rasta_interlocking.cfg \
0.0.0.0:4242 127.0.0.1 9998 127.0.0.1 9999 97 96 127.0.0.1:50001 &
INTERID=$!
cargo run --bin sender 0.0.0.0 50001 config/pin_config.toml