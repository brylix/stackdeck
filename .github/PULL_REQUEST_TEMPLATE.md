## What and why

<!-- One concern per PR. Refactors bundled with behaviour changes get bounced. -->

## How it was verified

- OS / version:
- Ran:
- Before:
- After:

## Checklist

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo check -p stackdeck-supervisor --target x86_64-pc-windows-msvc` (if `crates/supervisor` changed)
- [ ] No secrets, `target/`, or component binaries committed
- [ ] Listens on `127.0.0.1` unless exposure is deliberate
- [ ] No new UI runtime dependency
