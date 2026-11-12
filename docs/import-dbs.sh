#!/bin/sh
# Import the XAMPP dump into the Stackdeck MariaDB instance.
#
# MariaDB 13 renamed `utf8` to `utf8mb3` and dropped the bare
# `utf8_sinhala_ci` alias, so the 10.4 dump's DDL must be remapped to
# utf8mb4 before it will load. Collation only affects comparison/sorting;
# the stored bytes are unchanged.
#
# System schemas are excluded: importing the old `mysql` DB would replace the
# accounts and root password created on the new server.
set -uo pipefail

DUMP="$HOME/Backups/xampp-2026-11-12/databases.sql.zst"
SOCK=/tmp/stackdeck.sock
CLIENT=/opt/homebrew/opt/mariadb/bin/mariadb
P="$(security find-generic-password -s stackdeck-mariadb -w)"
export MYSQL_PWD="$P"
WORK=/tmp/sd-import
LOG="$WORK/import.log"
SKIP=" information_schema mysql performance_schema "

rm -rf "$WORK"; mkdir -p "$WORK"
exec >>"$LOG" 2>&1
say() { echo "[$(date '+%H:%M:%S')] $*"; }

run() { "$CLIENT" -u root -S "$SOCK" --default-character-set=utf8mb4 "$@"; }

# 1. Clear any partial state from a previous attempt.
say "dropping existing schemas"
for db in $(run -N -e "select schema_name from information_schema.schemata" | tr -d '\r'); do
  case "$SKIP" in *" $db "*) continue;; esac
  run -e "DROP DATABASE IF EXISTS \`$db\`"
done

# 2. Split per schema, remapping charsets. CREATE DATABASE lines are kept so
#    each DB keeps its original intent (now in utf8mb4).
say "splitting dump"
zstd -dc "$DUMP" | DIR="$WORK" perl -ne '
  s/utf8_sinhala_ci/utf8mb4_sinhala_ci/g;
  s/\butf8mb3_/utf8mb4_/g;
  s/\butf8_/utf8mb4_/g;
  s/\bCHARSET[ =]utf8\b/CHARSET=utf8mb4/g;
  s/\bCHARACTER SET utf8\b/CHARACTER SET utf8mb4/g;
  s/\bSET NAMES utf8\b/SET NAMES utf8mb4/g;
  s/DEFINER=`` //g;
  if (/^CREATE DATABASE.*?`([^`]+)`/) {
    close($O) if $O;
    open($O, ">", "$ENV{DIR}/$1.sql") or die $!;
    print {$O} "SET \@OLD_UNIQUE_CHECKS=\@\@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\n";
    print {$O} "SET \@OLD_FOREIGN_KEY_CHECKS=\@\@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n";
    print {$O} "SET \@OLD_SQL_MODE=\@\@SQL_MODE, SQL_MODE=\x27NO_AUTO_VALUE_ON_ZERO\x27;\n";
    print {$O} "SET \@OLD_TIME_ZONE=\@\@TIME_ZONE, TIME_ZONE=\x27+00:00\x27;\n";
    print {$O} "SET \@OLD_CHARACTER_SET_CLIENT=\@\@CHARACTER_SET_CLIENT;\n";
    print {$O} "SET \@OLD_CHARACTER_SET_RESULTS=\@\@CHARACTER_SET_RESULTS;\n";
    print {$O} "SET \@OLD_COLLATION_CONNECTION=\@\@COLLATION_CONNECTION;\n";
    print {$O} "SET \@OLD_SQL_NOTES=\@\@SQL_NOTES;\n";
    print {$O} "SET NAMES utf8mb4;\n";
  }
  print {$O} $_ if $O;
'
say "split into $(ls "$WORK"/*.sql 2>/dev/null | wc -l | tr -d ' ') files"

# 3. Import and verify table counts against the dump text.
fail=0
for file in "$WORK"/*.sql; do
  db=$(basename "$file" .sql)
  case "$SKIP" in *" $db "*) say "skip $db"; continue;; esac
  expected=$(grep -c '^CREATE TABLE' "$file")
  if run <"$file"; then
    actual=$(run -N -e "select count(*) from information_schema.tables where table_schema='$db' and table_type='BASE TABLE'" | tr -dc '0-9')
    if [ "${actual:-0}" -eq "$expected" ]; then
      say "OK   $db tables=$actual"
    else
      say "MISMATCH $db expected=$expected actual=${actual:-0}"
      fail=$((fail + 1))
    fi
  else
    say "FAILED $db"
    fail=$((fail + 1))
  fi
done

say "schemas now: $(run -N -e 'select count(*) from information_schema.schemata' | tr -dc '0-9')"
say "failures: $fail"
say "done"
