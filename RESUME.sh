#!/bin/sh
# Resume notes — XAMPP -> Stackdeck migration, 2026-11-13.
#
# Informational: prints state and safe checks. Changes nothing.
# Snapshot only; it does not update itself.

cat <<'NOTES'
STACKDECK — MIGRATION RESUME
============================
App repo:   ~/Documents/Projects/rust/stackdeck  (origin: brylix/stackdeck)
Installed:  /Applications/Stackdeck.app   (signed + notarized, v0.1.0)

DATA
  Projects:  ~/Sites            (299,857 files, was XAMPP htdocs)
  Backup:    ~/Backups/xampp-2026-11-12/  (2.8G, sha256 verified, PRISTINE)
  Database:  MariaDB 13, 48 schemas / 4041 tables imported and verified
  XAMPP:     still installed (~9.5G, incl. 4.3G stale datadir); stopped

CREDENTIALS (Keychain)
  stackdeck-mariadb   root password for MariaDB
  stackdeck-notarize  Apple app-specific password for notarization

SERVICES (start from the app, never from a shell)
  MariaDB   -> 127.0.0.1:3306, socket /tmp/stackdeck.sock
  PHP       -> presets 8.5 / 8.3 / 8.1, unix socket each
  Apache    -> 127.0.0.1:8080, serves ~/Sites, honours .htaccess
  phpMyAdmin-> 127.0.0.1:8081 (php@8.3)

  Port 80 is redirected to 8080 by pf. Apache stays user-owned so the app can
  stop it. pf is a deliberate trade: it means a system packet filter is on.

PROJECT PHP VERSIONS
  8.1  Laravel 8/9  (most projects: bowwow, kings, crimto, finedepot, ...)
  8.3  Laravel 10/11
  8.5  new work only — breaks Laravel <= 10
  erp is plain PHP, no framework: 8.1.

DONE
  Blank DB root replaced by generated password; phpMyAdmin cookie auth
  Apache 2.4.71 / PHP 8.1-8.5 / MariaDB 13 replace 2.4.56 / 8.2.4 / 10.4
  Per-app trees, dotfiles denied, .htaccess honoured
  XAMPP removed from PATH; app signed, notarized, released as pre-release

PENDING
  .env DB_PASSWORD still blank in ~13 apps (erp patched directly)
  Intel (x86_64) build not produced
  CI signing secrets not set; p12 never exported
  Branch protection + topics unset on the repo
  Unrelated audit items: firewall off, TeamViewer 15.76.6, macOS 27.0.1,
    unsigned third-party apps, six remote-access tools
  XAMPP still installed; old var/mysql (4.3G) reclaimable after validation
NOTES

echo
echo "--- live checks ---"
for p in 3306 8080 8081; do
  if lsof -nP -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "port $p: listening"
  else
    echo "port $p: free"
  fi
done

printf "%s: " "sites files"; find "$HOME/Sites" -type f 2>/dev/null | wc -l | tr -d ' '
printf "backup integrity: "
( cd "$HOME/Backups/xampp-2026-11-12" 2>/dev/null && shasum -a 256 -c SHA256SUMS >/dev/null 2>&1 && echo ok || echo FAILED )

printf "mariadb password present: "
security find-generic-password -s stackdeck-mariadb -w >/dev/null 2>&1 && echo yes || echo MISSING

printf "unmanaged processes: "
pgrep -f "opt/mariadb/bin/mariadbd|opt/httpd/bin/httpd|php-fpm: master" 2>/dev/null | wc -l | tr -d ' '
