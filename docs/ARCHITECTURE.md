# Architecture

## Shape

```
ui/  ──invoke──▶  src-tauri (thin)  ──▶  crates/supervisor (privileged logic)
```

The webview holds no privileges beyond calling named commands. Everything
that touches the filesystem, network or child processes lives in the
supervisor crate, which has no Tauri dependency and is unit-tested.

## Decisions

**System webview, not Chromium.** Tauri uses WKWebView on macOS and WebView2 on
Windows. Smaller, and the OS keeps it patched — at the cost of an untrusted
renderer. Hence: capabilities are scoped to the `main` window, `remote.urls` is
empty, CSP is set, and `withGlobalTauri` is the only convenience we allow.

**`*.localhost`, not `*.test`.** `localhost` subdomains resolve to loopback
natively in every current browser on both target OSes. No dnsmasq, no hosts-file
writes, no admin rights for DNS.

**No shared docroot.** One generated Caddyfile fragment per site. `.env` and
`.git` are denied in the generated config rather than by convention.

**Loopback first.** Servers bind `127.0.0.1`. Exposing to the LAN is an explicit
per-site toggle that rewrites the fragment and reloads Caddy.

**High ports by default.** 8080/8443; binding 80/443 is opt-in behind one
elevated helper.

**Per-OS PHP SAPI.** PHP-FPM does not exist on Windows:

| OS | SAPI | Front end |
|---|---|---|
| macOS | `php-fpm` (unix socket) | Caddy `php_fastcgi` |
| Windows | `php-cgi.exe` (TCP 9123) | Caddy `php_fastcgi` |

**Process tree teardown.** Unix: own process group, `SIGTERM` to `-pgid`, then
`SIGKILL`. Windows: job object with `KILL_ON_JOB_CLOSE`, so a supervisor crash
still reaps `mysqld`. Without this, the data directory and port stay held.

## Trust boundaries

| Boundary | Control |
|---|---|
| Webview → app | named commands, window-scoped capability |
| App → downloads | pinned version + SHA-256 per artifact |
| Update channel | minisign signature (`tauri-plugin-updater`) |
| LAN → servers | loopback bind by default, explicit opt-in |
| Sites → each other | separate vhost roots, no directory indexing |

## Non-goals

- Production hosting
- Linux (WebKitGTK patch lag is a separate problem)
- Bundling a browser engine

## Open items

1. Elevated helper for ports 80/443
2. Signed updater keys + CI release pipeline
3. Import wizard (dump existing DBs, rewrite `.env`, move docroots to `~/Sites`)
4. Per-component resolve/verify wiring (`manifest::resolve` → downloader)
