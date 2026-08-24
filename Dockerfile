# PrivZapp — build the WASM web bundle, serve it as pure static files.
#
# There is deliberately no application server in the runtime image: all file
# processing happens in the user's browser (ADR-0001). nginx here only hands
# out bytes.

# Fully-qualified image names so Podman resolves them without a
# registries.conf; Docker treats them identically.

# ---- build stage ----------------------------------------------------------
# trixie (not bookworm): the prebuilt dx binary needs glibc >= 2.39.
FROM docker.io/library/rust:1.98-slim-trixie AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# dioxus-cli pinned to the version the workspace is developed against;
# binstall fetches the prebuilt binary instead of compiling for ~10 min.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y dioxus-cli@0.7.10

WORKDIR /src
COPY . .

# Canonical origin baked into the prerendered SEO pages and sitemap.
# Override for your deployment: docker build --build-arg BASE_URL=https://your.domain
ARG BASE_URL=https://privzapp.com
ENV BASE_URL=$BASE_URL

# Same script contributors run locally: dx release build + PWA files +
# prerendered SEO pages (per-tool HTML, sitemap.xml, robots.txt).
RUN ./scripts/build-web.sh

# Precompress the bundle once at build time; nginx (gzip_static) serves
# the .gz siblings directly — the multi-MB wasm shrinks ~3x with zero
# per-request CPU.
RUN cd target/dx/privzapp/release/web/public \
    && find . -type f \( -name '*.wasm' -o -name '*.js' -o -name '*.mjs' \
         -o -name '*.css' -o -name '*.html' -o -name '*.json' \
         -o -name '*.svg' -o -name '*.webmanifest' -o -name '*.xml' \
         -o -name '*.txt' \) -exec gzip -9 -k {} +

# ---- runtime stage --------------------------------------------------------
FROM docker.io/library/nginx:1.27-alpine

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY deploy/security-headers.conf /etc/nginx/pz-security-headers.conf
COPY --from=builder /src/target/dx/privzapp/release/web/public /usr/share/nginx/html

EXPOSE 80
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD wget -q --spider http://127.0.0.1/ || exit 1
