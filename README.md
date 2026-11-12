# Stackdeck

A local dev-stack manager: pinned PHP, MariaDB and Caddy, per-site virtual
hosts on `*.localhost`, loopback-only by default, credentials generated on
first run.

Built because XAMPP's bundled Apache 2.4.56 / PHP 8.2.4 / OpenSSL 1.1.1t are
frozen at install time and bind `0.0.0.0` with a blank database root password.

## Layout

| Path | Purpose |
|---|---|
| `crates/supervisor` | Process supervision, component manifest, health checks (no Tauri) |
| `src-tauri` | Tauri shell, commands, capabilities |
| `ui` | Static HTML + vanilla JS — no Node build step |
| `manifest` | Pinned component versions with SHA-256 |

## Principles

1. Webview is untrusted; the supervisor is privileged.
2. Loopback by default; exposing to the LAN is explicit.
3. One Caddyfile fragment per site, never a shared docroot.
4. Dotfiles (`.env`, `.git`) denied at the server, not by convention.
5. Every component download is hash-verified.

## Develop

```bash
cargo check -p supervisor     # logic crate, fast
cargo tauri dev               # full app
```

Component binaries are not committed. On macOS they come from Homebrew; on
Windows from the pinned manifest.

Sites live in `~/Sites` — **not** under `Documents`, which is TCC-protected
and would force Full Disk Access for the server.
