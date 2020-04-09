# TCN Protocol Simulator + In-Memory Backend Server

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

## `tcn_server`

The server has two routes:

- `POST /submit` with the binary encoding of a TCN 0.4 report to submit a
  report;

- `GET /get_reports/{n}` where `n` is the string encoding of a time interval
  index, computed as `unixtime / time_interval`.

The `time_interval` is a deployment parameter, controlled by a command-line flag.

These routes should be changed in the future as the backend API evolves.

Server performance could be improved by changing the storage mutex to an RWLock
(to handle multiple read requests) and changing the accumulator to an unbounded
channel (so that writes are decoupled from reads), effectively using an async
channel as a data store.  However, performance is not currently an issue.

## `simulator`

The simulator has a number of parameters, which can be accessed by `--help`.

The simulator is structured as follows.  Each user is a separate async task,
running concurrently on a Tokio threadpool.  Bluetooth broadcasts are simulated
by a Tokio `broadcast` channel shared by all users.  Currently, users receive
each TCN broadcast by all other users with a fixed probability.   Users submit
reports over HTTP to the backend server and make HTTP requests to the backend
server to monitor for new reports from other users.

In the future, the simulator could be changed in the future to encode more
complex logic:

- users moving around in space and receiving broadcasts from users "nearby" to
  them, rather than randomly receiving broadcasts from all users;

- malicious users deviating from the protocol (submitting invalid reports,
  trying to submit other user's TCNs, etc);

- passive adversaries attempting to reconstruct users' location tracks;

These could be used to perform empirical simulations on the protocol's behavior.

The simulator has a few barriers to scalability, notably:

- all user tasks independently download the report batch and expand it into the
  candidate TCNs before matching the candidates with the observed TCNs.
  Instead, a proxy task could periodically fetch reports and expand them once,
  then pass a reference to the candidate TCNs to all of the user tasks.

