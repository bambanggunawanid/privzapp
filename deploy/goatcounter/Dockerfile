# GoatCounter sidecar for PrivZapp's anonymous page counting (ADR-0012).
#
# Self-hosted on purpose: visit counts must never involve a third party.
# The binary is fetched version- and sha256-pinned, same contract as the
# app's other runtimes (ffmpeg, tesseract). sqlite3 is only for the
# entrypoint to enforce the privacy collection settings on every boot.
FROM docker.io/library/alpine:3.20

ARG GC_VERSION=v2.7.0
ARG GC_SHA256=98d221cb9c8ef2bf76d8daa9cca647839f8d8b0bb5bc7400ff9337c5da834511

RUN apk add --no-cache ca-certificates sqlite \
    && wget -q -O /tmp/gc.gz "https://github.com/arp242/goatcounter/releases/download/${GC_VERSION}/goatcounter-${GC_VERSION}-linux-amd64.gz" \
    && echo "${GC_SHA256}  /tmp/gc.gz" | sha256sum -c - \
    && gunzip /tmp/gc.gz \
    && install -m 0755 /tmp/gc /usr/local/bin/goatcounter \
    && rm -f /tmp/gc

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
