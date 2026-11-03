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
RUN cargo build --release --locked -p supa-api --bin supa-api --bin supa-staging-schema

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:08476c244888bfb425bae1ae4dd7c17f093e972aab8426883d97f3c6f280f7c1 AS runtime
# Official minimal glibc runtime: no package manager, shell or account utilities.
# Runtime ELF/library, CA, non-root and vulnerability checks remain mandatory in CI.
ENV SUPA_PRIVATE_STAGING=true SUPA_BIND=[::]:8080 PORT=8080 SUPA_PROVIDER=offline SUPA_API_SCHEDULER_ENABLED=false SUPA_WALLET_ENABLED=false
COPY --from=build /app/target/release/supa-api /usr/local/bin/supa-api
COPY --from=build /app/target/release/supa-staging-schema /usr/local/bin/supa-staging-schema
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/supa-api"]
