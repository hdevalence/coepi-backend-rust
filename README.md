# CoEpi Backend - Rust

Rust implementation of [CoEpi](https://github.com/Co-Epi) backend - for distributed contact tracing. 

## How to deploy

You'll need:
- [Rust 1.41 or newer](https://www.rust-lang.org/learn/get-started)
- [Redis](https://redis.io/)

Once you've installed those, 
1) start up Redis by running `redis-server`
2) start up the HTTP server with `cargo run --release`.

You'll now have an HTTP server listening on port 8080.
