#!/bin/sh
# First boot: create the site. Every boot: enforce the privacy contract.
set -eu

DB="sqlite+/data/goatcounter.db"
VHOST="${GC_VHOST:-stats.privzapp.com}"

if [ ! -f /data/goatcounter.db ]; then
  : "${GC_EMAIL:?set GC_EMAIL (dashboard login) in .env}"
  : "${GC_PASSWORD:?set GC_PASSWORD (dashboard login) in .env}"
  goatcounter db create site \
    -vhost "$VHOST" \
    -user.email "$GC_EMAIL" \
    -password "$GC_PASSWORD" \
    -db "$DB" -createdb
fi

# The privacy contract, enforced structurally on EVERY start so a click
# in the dashboard can't silently widen collection: store the page path
# and the visitor's country - nothing else. collect=32 is GoatCounter's
# "Location (country)" bit alone: no sessions/uniques (so the daily
# IP+UA hash is never computed), no user-agent/browser stats, no screen
# size, no language, no referrer, no region.
sqlite3 /data/goatcounter.db \
  "update sites set settings = json_set(settings, '\$.collect', 32, '\$.collect_regions', '');"

exec goatcounter serve -listen :8080 -tls none -db "$DB" -store-every 10
