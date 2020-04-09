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

## Example

Example log output using `RUST_LOG=info`.

Simulator output snippet:
```
Apr 09 14:06:44.142  INFO run{id=5}:send_reports: sending reports from most recent raks raks_to_report=14
Apr 09 14:06:44.160  INFO run{id=13}:send_reports: sending reports from most recent raks raks_to_report=14
Apr 09 14:06:44.171  INFO run{id=19}:send_reports: sending reports from most recent raks raks_to_report=14
Apr 09 14:06:51.252  INFO run{id=1}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("290bc97f44b7e160c4953d15129b80f5")
Apr 09 14:06:51.252  INFO run{id=1}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("7ea9519d7f585e8c929dd28a6928fe70")
Apr 09 14:06:51.265  INFO run{id=4}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("c8277708270637b19af68560b64755c9")
Apr 09 14:06:51.288  INFO run{id=10}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("b5894aacbe89184e90b810c79e5d0dbe")
Apr 09 14:06:51.292  INFO run{id=19}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("8469311ba8d765d88e96a2a850dc1d90")
Apr 09 14:06:51.292  INFO run{id=19}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("f971b0d130ce6a1f2c384a0bf4814e0a")
Apr 09 14:06:51.297  INFO run{id=12}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("3af3262d27fff8ba894115f9cfeb8ccd")
Apr 09 14:06:51.302  INFO run{id=18}:fetch_reports: got report about observed tcn tcn=TemporaryContactNumber("93b3c62d1ae66a82e2b3c9451415246d")
```

Server output snippet:
```
Apr 09 14:06:44.170  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("ae2d92bf1c04f3d6af144972e94cc5ee26d8959671b0dc89da6d269ec7eb0012"), tck_bytes: "46b95c460206071cc76d7898a78e9db565faf367c07b9cb8d2d59ad45328b837", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "2ec47effda3b59700254d2df9b745fe49df8c534e046fae47bb2a64de98b6b1c", s_bytes: "0b1284d999aaae5f94519d023fd65023277aebf288409b5952de6c2dab0bf70b" } }}: got report
Apr 09 14:06:44.171  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("a3995962b0d22700e7108600de4592c35cbf11ccdd1be5fba237ecda4dfb0d4d"), tck_bytes: "f5253258f119eab3ca7ce6786b3a27f2afd0662216e72c1251acefee722f3737", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "d082b4c478e54af8bee40cb1306e3b30442cc89926c771d0a66783e44bf324d1", s_bytes: "168dd1bffff8e40b5dea7f91c7a2590b2606b5e0801ede5a792eaf50c7f7f204" } }}: got report
Apr 09 14:06:44.171  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("bb6c5c61ba7c5c1a047fea7f42ffe2c712daa0e8b9befa83a225a94022ca8499"), tck_bytes: "9a128d8cad9fc219bc9637605f6a250d7991ff9402a47171cc8c3ccb6818fa92", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "ecd713a32304dbb5874f159c1f859821af67b9e1bb28b3aec7bafb158aae9d2e", s_bytes: "1dc127c74e271cce9c58d51572c2bf20109fab22e9c260551bdfa5e07b9d770d" } }}: got report
Apr 09 14:06:44.180  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("820db226f56791eb4d841d6bf1644514fe0fb59352981224d9daeed6c27bace0"), tck_bytes: "dd8d08eadd141358049d4ed64271c7ce2a49ce9a40dbca990ffd23105c765cf5", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "d4cd70133bc0226bb0b991f003db171e4120a9633acb67f7b6867d08bc25817e", s_bytes: "87f39f71e517b6a640de0ba77ce7336d12f43700fdc3cd0cf67a549c639b0d08" } }}: got report
Apr 09 14:06:44.180  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("1f5cc5e9b22cae8abcb7caa83c7a1e12ff4f86f8dfcbdd61528dae5e19bcbea3"), tck_bytes: "ec0b2411784bb47883572854c82556acd46b68c680bf37f172fa094f16d9600d", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "adcf0f9d05b9de686c479cd117e8f00a2a8ada026ba8cb55bcafe4412afcd84c", s_bytes: "72ea05fb301eb16b873d8ff4d060cd537a821c708e131e66d1595921c280160b" } }}: got report
Apr 09 14:06:44.181  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("c3db60eaf8b3d870d38f74df03c21e88ba89e706a783cd1e3e0d1c0f297d8f94"), tck_bytes: "4cefb4351cc2fc6970a8efab2dac1218ae47546d1480d0ee636b52cab3efc73e", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "374fce81403dcaa2f1c70e3c43dab30a50c225b43d13a9fd15c7b25c5d05fa81", s_bytes: "8f8de0d63b1c6cb5016a7d183554d0756eb6e5aa84bdfde14be1453778054b08" } }}: got report
Apr 09 14:06:44.181  INFO save{report=SignedReport { report: Report { rvk: PublicKeyBytes("d50a95c763f6e760f02adfc938f2d12cf8bd3a45962d63ae72ed5e893f7672c7"), tck_bytes: "1acacae0be1870e21771360bb1d30633e4d50016b42a7f84f64b9fd023a3b359", j_1: 1, j_2: 288, memo_type: CoEpiV1, memo_data: "" }, sig: Signature { R_bytes: "d47364bf28d8d99cb032c69b548d40d499eab4e4610a943fab01bbac78c33fb4", s_bytes: "a0f1c60fa7e6d279a6b6a368d089026e8e63e4be034933217a787b589ba13a0d" } }}: got report
Apr 09 14:06:51.227  INFO get{timeframe=ReportTimestamp(264411067)}: sealed reports into byte buffer count=12 num_bytes=1608
```

