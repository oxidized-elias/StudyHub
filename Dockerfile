FROM rust:1-bookworm AS build
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY web ./web
COPY apps ./apps
RUN cargo build --locked --release -p study-hub

FROM node:22-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends poppler-utils ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /source/target/release/study-hub ./target/release/study-hub
COPY server.mjs package.json package-lock.json ./
RUN mkdir -p data && chown -R node:node /app
USER node
ENV STUDYHUB_BIND=0.0.0.0:8082 DATA_DIR=/app/data
EXPOSE 8082
VOLUME ["/app/data"]
CMD ["node", "server.mjs", "--release", "--no-build"]
