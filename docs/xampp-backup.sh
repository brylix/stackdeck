#!/bin/sh
# Back up XAMPP projects + all databases, verify, then stop XAMPP services.
set -uo pipefail

X=/Applications/XAMPP/xamppfiles
DEST="$HOME/Backups/xampp-$(date +%Y-%m-%d)"
LOG="$DEST/backup.log"
mkdir -p "$DEST" || exit 1

exec >>"$LOG" 2>&1
say() { echo "[$(date '+%H:%M:%S')] $*"; }

say "backup start -> $DEST"

# 1. Databases (consistent snapshot; no table locks on InnoDB).
say "dumping databases"
"$X/bin/mysqldump" -u root --all-databases --single-transaction --quick \
  --routines --events --triggers --default-character-set=utf8mb4 \
  | zstd -T0 -3 -q -o "$DEST/databases.sql.zst"
say "dump exit=$? size=$(du -h "$DEST/databases.sql.zst" | cut -f1)"

# 2. Projects.
say "archiving htdocs"
tar -cf - -C "$X" htdocs | zstd -T0 -3 -q -o "$DEST/htdocs.tar.zst"
say "archive exit=$? size=$(du -h "$DEST/htdocs.tar.zst" | cut -f1)"

# 3. Configuration worth restoring alongside.
say "archiving config"
tar -czf "$DEST/xampp-config.tgz" -C "$X" \
  etc/httpd.conf etc/my.cnf etc/extra/httpd-vhosts.conf \
  phpmyadmin/config.inc.php 2>/dev/null
say "config exit=$?"

# 4. Restore notes + inventory.
"$X/bin/mysql" -u root -N -e \
  "select schema_name from information_schema.schemata order by 1" \
  >"$DEST/databases.txt"
find "$X/htdocs" -type f 2>/dev/null | wc -l >"$DEST/file-count.txt"

cat >"$DEST/RESTORE.md" <<'EOF'
# Restore

1. Verify: `cd $(dirname $0); shasum -a 256 -c SHA256SUMS`
2. Projects: `zstd -dc htdocs.tar.zst | tar -xf - -C /Applications/XAMPP/xamppfiles`
3. Databases: `zstd -dc databases.sql.zst | mysql -u root`
4. Config: `tar -xzf xampp-config.tgz -C /Applications/XAMPP/xamppfiles`

The dump is a full `--all-databases`; it recreates every schema.
EOF

# 5. Checksums (after the files are closed).
say "checksums"
( cd "$DEST" && shasum -a 256 databases.sql.zst htdocs.tar.zst >SHA256SUMS )

# 6. Verify integrity and legibility.
say "verify: zstd test"
zstd -t "$DEST/databases.sql.zst" && say "db archive OK"
zstd -t "$DEST/htdocs.tar.zst" && say "htdocs archive OK"
say "schemas in dump: $(zstd -dc "$DEST/databases.sql.zst" | grep -c '^CREATE DATABASE')"
say "files in manifest: $(cat "$DEST/file-count.txt")"
say "total size: $(du -sh "$DEST" | cut -f1)"

say "backup complete"
