# SuperDrop

Experimental MIT-licensed Quick Share implementation for desktop computers.

**Current status: QR activation and receiver discovery diagnostic only. File sending and receiving are not implemented. No Android or desktop compatibility is certified.**

Requires Rust stable and a LAN permitting multicast DNS and peer traffic:

```sh
cargo run -p superdrop -- activate
```

Scan the terminal QR using your Android camera. The command waits up to two minutes for a receiver advertisement bound to that session, then prints its name and port. Keep terminal output private. A discovered receiver is not a completed transfer.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

See [protocol decisions and remaining work](docs/protocol.md) and [the complete plan](PLAN.md). The plan requires real-phone transfers before expanding the desktop UI. If discovery fails, check the devices use the same network, disable interfering VPN routes, allow multicast DNS through the firewall, and avoid isolated guest networks.
