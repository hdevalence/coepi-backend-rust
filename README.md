# WIP code

This is a work-in-progress repo with two components:

- `tcn_server`: an in-memory backend server for the TCN protocol.

- `simulator`: a binary that simulates multiple users running the protocol and
  communicating with the server.  This simulates the protocol as well as
  load-tests the server.

Try
```
export RUST_LOG="tcn_server=debug,simulator=debug"
cargo run --release --bin tcn_server -- --help
cargo run --release --bin simulator -- --help
```
and play with the simulation parameters

