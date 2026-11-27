# syntax=docker/dockerfile:1
#
# One image for the whole service: the backend (HTTP and gRPC) and the federated frontend, each on
# its own port. Kept small: Alpine, one static Rust binary, the Node executable alone, and only the
# files Next.js traced for its standalone server. Logs and generated files go to the /data volume.
#
# The build context is this repository, wherever it is checked out. From its root:
#
#   docker build --secret id=GIT_AUTH_TOKEN -t srv-iam .
#
# Shared code comes from GitHub, downloaded by the package managers: zerpto-core and zerpto-proto at
# the tags in service/Cargo.toml, @zerpto/ui and @zerpto/sdk within the ranges in
# federation/package.json (the commits are in Cargo.lock and pnpm-lock.yaml). For private repositories, the GIT_AUTH_TOKEN
# secret is a token that can read them; it is never stored in the image. What is sent to the builder
# is limited by Dockerfile.dockerignore, beside this file. In the build stages this repository is
# placed in a directory `iam`.

ARG BIN=iam
# At least the highest `rust-version` in service/Cargo.lock (loco-rs, sea-orm and sqlx need 1.94).
ARG RUST_VERSION=1.94
# federation/package.json needs Node 22.12 or later. Corepack ships with Node up to 24.
ARG NODE_VERSION=24
# Of the Node image and the runtime image; they must match, as the Node executable moves between them.
ARG ALPINE_VERSION=3.22

# --- Backend -----------------------------------------------------------------
# Built on Alpine, the binary is statically linked (musl).
FROM rust:${RUST_VERSION}-alpine AS backend
ARG BIN
ARG TARGETARCH
# Optional commit id, compiled into the app's version and the Sentry release.
ARG BUILD_SHA

# A C toolchain for the native crates; protoc for the build.rs of zerpto-core and zerpto-proto
# (protobuf-dev has google/protobuf/*.proto); git for Cargo to download those two.
RUN apk add --no-cache build-base cmake git linux-headers perl protobuf protobuf-dev

COPY service/ /src/iam/service/
# Compiled into the binary (src/extensions/core_manifests.rs).
COPY federation/manifest.yaml /src/iam/federation/
WORKDIR /src/iam/service

# Dependencies and incremental output live in BuildKit cache mounts. The binary is copied out,
# because a cache mount is not part of the layer. The profile settings trade build time for size.
# Cargo fetches with the git command, which asks the credential helper below for the token. The
# helper reads the secret file, so the token is in no layer, URL or build argument.
RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/cargo-target,sharing=locked \
    export CARGO_TARGET_DIR="/cargo-target/iam-musl-${TARGETARCH}" \
        CARGO_PROFILE_RELEASE_STRIP=true \
        CARGO_PROFILE_RELEASE_LTO=true \
        CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
        CARGO_PROFILE_RELEASE_OPT_LEVEL=s \
        CARGO_NET_GIT_FETCH_WITH_CLI=true \
        GIT_TERMINAL_PROMPT=0 \
        GIT_CONFIG_COUNT=1 \
        GIT_CONFIG_KEY_0=credential.https://github.com.helper \
        GIT_CONFIG_VALUE_0='!f() { test "$1" = get && test -s /run/secrets/GIT_AUTH_TOKEN && printf "username=x-access-token\npassword=%s\n" "$(cat /run/secrets/GIT_AUTH_TOKEN)"; }; f' \
    && cargo build --release --locked --bin "${BIN}" \
    && install -D -m 0755 "${CARGO_TARGET_DIR}/release/${BIN}" /out/iam

# --- Frontend build ----------------------------------------------------------
FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS web-build
ENV COREPACK_HOME=/opt/corepack \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    NEXT_TELEMETRY_DISABLED=1 \
    pnpm_config_verify_deps_before_run=false
# The pnpm version is the `packageManager` field of package.json.
RUN corepack enable

# git for pnpm to download @zerpto/ui and @zerpto/sdk.
RUN apk add --no-cache git

# Manifests first, so source changes keep the dependency layers.
COPY federation/package.json \
     federation/pnpm-lock.yaml \
     federation/pnpm-workspace.yaml \
     /app/iam/federation/

# git asks the credential helper for the token, as in the backend stage, and reaches GitHub over
# HTTPS even where the lockfile records an SSH address. @zerpto/sdk is built during the install, by
# its `prepare` script (allowed in pnpm-workspace.yaml).
WORKDIR /app/iam/federation
RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    export GIT_TERMINAL_PROMPT=0 \
        GIT_CONFIG_COUNT=3 \
        GIT_CONFIG_KEY_0=credential.https://github.com.helper \
        GIT_CONFIG_VALUE_0='!f() { test "$1" = get && test -s /run/secrets/GIT_AUTH_TOKEN && printf "username=x-access-token\npassword=%s\n" "$(cat /run/secrets/GIT_AUTH_TOKEN)"; }; f' \
        GIT_CONFIG_KEY_1=url.https://github.com/.insteadOf \
        GIT_CONFIG_VALUE_1=ssh://git@github.com/ \
        GIT_CONFIG_KEY_2=url.https://github.com/.insteadOf \
        GIT_CONFIG_VALUE_2=git@github.com: \
    && pnpm install --frozen-lockfile --store-dir /pnpm/store

COPY federation/ ./
# Rsbuild writes public/mf/, then `next build` writes .next/. NEXT_PRIVATE_STANDALONE is Next's
# switch for `output: "standalone"`, set here so next.config.ts stays the same for local work.
# The result is gathered in /out, laid out as the standalone server expects it.
RUN NEXT_PRIVATE_STANDALONE=true pnpm run build \
    && server=".next/standalone/iam/federation" \
    && { test -f "${server}/server.js" || { echo "no standalone server in ${server}" >&2; exit 1; }; } \
    && mkdir -p /out \
    && cp -a .next/standalone/. /out/ \
    && out="/out/iam/federation" \
    && cp -a public "${out}/" \
    && cp -a .next/static "${out}/.next/" \
    && cp extension.json manifest.yaml "${out}/" \
    && rm -rf "${out}/.next/cache" \
    && ln -s /data/cache "${out}/.next/cache" \
    && find /out -name '*.map' -delete

# --- Node executable ---------------------------------------------------------
# The Node image without npm, Corepack and Yarn: only the executable is copied from it.
FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS node

# --- Runtime -----------------------------------------------------------------
FROM alpine:${ALPINE_VERSION} AS runtime
# Documentation only (EXPOSE). The port the frontend listens on is `port` in extension.json, or the
# FRONTEND_PORT environment variable at run time.
ARG FRONTEND_PORT=3001

# libstdc++ for Node, bash for the entrypoint, tini to reap processes and pass signals on,
# ca-certificates for outgoing TLS.
RUN apk add --no-cache bash ca-certificates libstdc++ tini \
    && adduser -D -u 1000 node \
    && install -d -o node -g node /data /data/logs /data/cache

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    LOCO_ENV=production \
    FEDERATION_DIR=/app/iam/federation \
    LOG_DIR=/data/logs

COPY --from=node /usr/local/bin/node /usr/local/bin/node

# Backend: the binary, and config/ relative to its working directory.
COPY --from=backend /out/iam /usr/local/bin/iam
COPY service/config/production.yaml /app/service/config/

# Frontend: the standalone server with its traced dependencies.
COPY --from=web-build /out/ /app/

COPY --chmod=0755 docker/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY docker/healthcheck.mjs /app/healthcheck.mjs

WORKDIR /app/service
USER node
# Logs and the Next.js cache: what the container writes at run time. Uploads go to R2.
VOLUME ["/data"]

# Backend HTTP (PORT), backend gRPC (GRPC_PORT), frontend (FRONTEND_PORT).
EXPOSE 4001 5001 ${FRONTEND_PORT}

# The start period covers migrations on boot.
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD ["node", "/app/healthcheck.mjs"]

# `serve` runs both servers; any other command runs as given (see entrypoint.sh).
ENTRYPOINT ["/sbin/tini", "--", "/usr/local/bin/entrypoint.sh"]
CMD ["serve"]
