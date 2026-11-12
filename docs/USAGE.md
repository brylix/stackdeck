# Using Stackdeck

Stackdeck runs the local services your projects need — MariaDB, PHP-FPM,
phpMyAdmin — and provisions them with generated credentials. Servers are
loopback-only and are torn down with the app.

**Not built yet:** per-site vhosts and Caddy. Stackdeck currently manages
processes, not sites. There is no `myproject.localhost` yet.

## First run

1. In **Setup**, each service reads *not installed*.
2. **Install** — Homebrew fetches it; the row updates itself.
3. **Set up** — credentials are generated and stored in Keychain.
4. All rows read **ready**.

Setup is idempotent and keeps an existing password.

## Start order

MariaDB → PHP → phpMyAdmin. Pick a preset, press **Start**, press **Stop**
when done. Quitting the app stops every child process.

Start MariaDB before anything needing a database; PHP alone serves nothing.

## Serving a site

Apache serves everything under `~/Sites` at `http://127.0.0.1:8080/<folder>/`.
Start MariaDB and the matching PHP preset, then the **Apache** preset.

Each project's `.htaccess` applies — `AllowOverride All` is set, so rewrites,
`ErrorDocument`, auth and `php_value` behave as they did under XAMPP. Dotfiles
(`.env`, `.git`) are denied in the server config.

To add a project, drop it in `~/Sites`; no server config change is needed.

Laravel apps need `/public/` appended: `.../bowwow/public/`.

## Limits of path hosting

- **Laravel apps need `/public/`** appended: `.../bowwow/public/`. Their
  `index.php` lives there, so the bare folder falls through to the root
  `index.php` and lands on the dashboard page instead.
- **One PHP version for every folder.** Apache pins 8.1; apps needing 8.3 or
  8.5 would need their own vhost (not built yet).
- **Cookies collide.** All folders share the `localhost` host, so apps using
  `PHPSESSID` overwrite each other's session — logging into one logs you out
  of another.
- **Absolute asset paths break** under a prefix; apps emitting `/css/app.css`
  resolve to the portal root, not `/erp/css/app.css`.

Use the portal for quick checks; keep a `.localhost` block per app for real
work, especially Laravel.

## Ports

| Service | Address | Bound |
|---|---|---|
| MariaDB | `127.0.0.1:3306`, socket `/tmp/stackdeck.sock` | all interfaces |
| Caddy (sites) | `http://<site>.localhost:8080` | loopback |
| phpMyAdmin | `http://127.0.0.1:8081` | loopback |
| PHP-FPM | unix socket | sockets |

MariaDB is deliberately reachable from the LAN, password-protected.
phpMyAdmin and FPM never leave the machine.

## Credentials

User `root`; the password is generated at setup:

```bash
security find-generic-password -s stackdeck-mariadb -w
```

Imported projects still carry an empty `DB_PASSWORD` in their `.env` — that
is manual work.

## PHP versions

| Preset | Use for |
|---|---|
| 8.5 | new work; breaks Laravel ≤ 10 |
| 8.3 | Laravel 10 / 11 |
| 8.1 | Laravel 8 / 9 legacy |

Each version has its own socket. A parse error on a 500 usually means the
wrong version.

## Files

```
~/Library/Application Support/Stackdeck/
  data/          MariaDB datadir
  phpmyadmin/    symlinks + generated config
  run/           FPM sockets, pid files
  log/           per-service out/err logs
  tmp/           phpMyAdmin scratch
```

## Troubleshooting

- **Address already in use** — something else holds the port, often XAMPP:
  `lsof -nP -iTCP:3306 -sTCP:LISTEN`
- **Service dies immediately** — read `log/<service>.err.log`.
- **Stuck on "not installed"** — run `brew install <formula>` to see the error.
- **`mysql: command not found`** — use `/opt/homebrew/opt/mariadb/bin/mariadb`;
  XAMPP still shadows the short name.

## Known limitations

- No Caddy or vhosts.
- phpMyAdmin on PHP's built-in server: single-threaded, HTTP only.
- macOS only; the Windows manifest is unused.
- Absolute paths in presets — source checkouts only.
- No import wizard.
