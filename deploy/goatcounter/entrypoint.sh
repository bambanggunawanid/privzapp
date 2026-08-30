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
# and the visitor's country - nothing else.
#
# collect is a bitmask; the values below are GoatCounter v2.7.0's
# (settings.go): Nothing=1 Referrer=2 UserAgent=4 ScreenSize=8
# Location=16 LocationRegion=32 Language=64 Session=128. We set exactly
# CollectLocation (16) - country - and nothing else, versus the stock
# default of 190 (referrer + user-agent + screen + location + region +
# session). So: no sessions/uniques (the daily IP+UA hash is never
# computed), no browser/OS stats, no screen size, no language, no
# referrer, and no sub-country region. Verify after upgrades: these are
# iota bitflags and could shift between major versions.
sqlite3 /data/goatcounter.db \
  "update sites set settings = json_set(settings, '\$.collect', 16, '\$.collect_regions', '');"

# GoatCounter logs the raw User-Agent of requests it classifies as bots
# (crawlers, headless browsers) so they can be excluded from the stats.
# We never look at that table, and a misclassified real visitor would
# have their UA sitting in it, so drop it on every start - retention is
# bounded to this container's uptime. Disclosed on the Privacy page.
sqlite3 /data/goatcounter.db "delete from bots;"

exec goatcounter serve -listen :8080 -tls none -db "$DB" -store-every 10
