# syntax=docker/dockerfile:1
FROM rust:1.99-bookworm AS build
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY web ./web
COPY apps/StudyHub/Cargo.toml ./apps/StudyHub/Cargo.toml
COPY apps/StudyHub/src ./apps/StudyHub/src
COPY apps/StudyHub/web ./apps/StudyHub/web
COPY apps/SelfHosted-AI_Tests/Cargo.toml ./apps/SelfHosted-AI_Tests/Cargo.toml
COPY apps/SelfHosted-AI_Tests/src ./apps/SelfHosted-AI_Tests/src
COPY apps/SelfHosted-AI_Tests/web ./apps/SelfHosted-AI_Tests/web
COPY logo.png ./logo.png
RUN cargo build --release --locked -p study-hub

FROM node:24-bookworm-slim AS runtime
ARG APP_UID=1000
ARG APP_GID=1000
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates poppler-utils imagemagick libheif-examples \
    && rm -rf /var/lib/apt/lists/* \
    && test "$APP_UID" -gt 0 && test "$APP_GID" -gt 0 \
    && groupmod -o -g "$APP_GID" node \
    && usermod -o -u "$APP_UID" -g "$APP_GID" node
WORKDIR /app
COPY --from=build /source/target/release/study-hub ./target/release/study-hub
COPY server.mjs package.json package-lock.json ./
COPY scripts/docker-healthcheck.mjs ./scripts/docker-healthcheck.mjs
RUN install -d -m 0700 -o node -g node /app/data /vault
ENV NODE_ENV=production \
    STUDYHUB_BIND=0.0.0.0:8082 \
    DATA_DIR=/app/data \
    LEARNING_DATA_DIR=/app/data/learning \
    VAULT_DIR=/vault
USER node
EXPOSE 8082
VOLUME ["/app/data", "/vault"]
HEALTHCHECK --interval=30s --timeout=7s --start-period=30s --retries=3 \
    CMD ["node", "/app/scripts/docker-healthcheck.mjs"]
STOPSIGNAL SIGTERM
CMD ["node", "server.mjs", "--release", "--no-build"]
