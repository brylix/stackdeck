# Contributing

Thanks for looking. Stackdeck is a small tool with a narrow purpose, so the
fastest way in is to pick a roadmap item rather than a new feature.

## Platform status

**macOS is supported. Windows is a target.**

Windows compiles at the supervisor level but is not usable: Homebrew is
macOS-only, so provisioning, paths, credentials and the SPA are all still
macOS-shaped. See the Windows roadmap in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before starting a port PR — the
pieces are listed in the order they need doing.

## Setup

Requires Rust, Xcode command line tools, and the Tauri CLI.

```bash
cargo install tauri-cli
rustup target add x86_64-pc-windows-msvc   # for the Windows compile check

cargo test --workspace
cargo tauri dev
```

Runtime services are optional for most work. If you need them installed
locally, the app's Setup panel will fetch PHP, MariaDB and Apache via
Homebrew — don't add them to the repo.

### The Windows compile check

The supervisor must keep compiling for Windows. Run this before opening a PR
that touches `crates/supervisor`:

```bash
cargo check -p stackdeck-supervisor --target x86_64-pc-windows-msvc
```

The app crate cannot be cross-checked from macOS (it needs `llvm-rc`); CI
builds it natively on `windows-latest`.

## The gates

All three must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Add tests next to the code. Logic belongs in `crates/supervisor`, which has no
Tauri dependency and is easy to test; `src-tauri` should stay thin command
plumbing.

## Layout

| Path | What goes here |
|---|---|
| `crates/supervisor` | Process supervision, manifest, health, provisioning |
| `src-tauri` | Commands and capability wiring only |
| `ui` | Static HTML/JS/CSS — no bundler, no Node |
| `manifest` | Pinned component versions with SHA-256 |
| `presets` | FPM pool configs, one socket per version |
| `docs` | Architecture, usage, examples |

## Principles

1. **Loopback by default.** Anything that listens must bind `127.0.0.1` unless
   a site is explicitly exposed.
2. **The webview is untrusted.** New commands need a capability scoped to the
   `main` window; a new remote origin must not get IPC.
3. **No credentials without generation.** Never ship a default password.
4. **Every download is hash-verified.** A mutable "latest" URL is a bug.
5. **Dotfiles stay private.** `.env` and `.git` are denied in server config.
6. **No new runtime for the UI.** If it needs Node, say why.

## Pull requests

- One concern per PR. Refactors bundled with behaviour changes get bounced.
- State what you ran, on which OS, and paste the relevant error before/after.
- Commit messages: imperative subject, short body explaining *why*.
- Don't commit secrets, `target/`, or generated component binaries.

By contributing you agree your work is licensed under the [MIT
LICENSE](LICENSE).

## Reporting bugs

Include the OS and version, what you did, what happened, and
`~/Library/Application Support/Stackdeck/log/<service>.err.log`. State whether
the service was started from the app or from a terminal — the app only
supervises what it spawned, and that distinction changes the diagnosis.
