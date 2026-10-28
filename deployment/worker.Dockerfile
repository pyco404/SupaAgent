# syntax=docker/dockerfile:1
FROM rust:1.93-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/domain/src crates/domain/src
COPY crates/persistence/Cargo.toml crates/persistence/Cargo.toml
COPY crates/persistence/src crates/persistence/src
COPY crates/providers/Cargo.toml crates/providers/Cargo.toml
COPY crates/providers/src crates/providers/src
COPY crates/capabilities/Cargo.toml crates/capabilities/Cargo.toml
COPY crates/capabilities/src crates/capabilities/src
COPY crates/runtime/Cargo.toml crates/runtime/Cargo.toml
COPY crates/runtime/src crates/runtime/src
COPY crates/api/Cargo.toml crates/api/Cargo.toml
COPY crates/api/src crates/api/src
COPY migrations migrations
RUN cargo build --release --locked -p supa-runtime --bin supa-runtime

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home supa
ENV SUPA_PRIVATE_STAGING=true SUPA_WORKER_BIND=[::]:8081 PORT=8081 SUPA_PROVIDER=offline SUPA_WORKER_ENABLED=false SUPA_WORKER_POLL_SECONDS=5
COPY --from=build /app/target/release/supa-runtime /usr/local/bin/supa-runtime
USER 10001
EXPOSE 8081
ENTRYPOINT ["/usr/local/bin/supa-runtime"]
