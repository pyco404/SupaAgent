# syntax=docker/dockerfile:1
# Candidate security overlay; no registry publication or database deployment.
FROM golang:1.27.3-trixie@sha256:c65ecfe613b539ac74c62a4c3b2c27e8f8a6076384c8f27e8a3c0f3be17cc385 AS gosu-build
ADD --checksum=sha256:33d7537d588ea49458b9509bcf4554bdf5ceacc66da71e5caa1058ea3b689c3b https://codeload.github.com/tianon/gosu/tar.gz/6456aaa0f3c854d199d0f037f068eb97515b7513 /tmp/gosu.tar.gz
WORKDIR /src
RUN tar -xzf /tmp/gosu.tar.gz --strip-components=1 \
    && go mod download && go mod verify \
    && CGO_ENABLED=0 GOTOOLCHAIN=local go build -mod=readonly -trimpath -ldflags='-s -w' -o /out/gosu . \
    && /out/gosu --version \
    && test "$(/out/gosu nobody id -u)" = 65534

FROM ghcr.io/railwayapp-templates/postgres-ssl:18@sha256:4fc833b43d2aaf6ecc9b18615f5533602b723a32d0e2efe2dae8ca0f7217c863 AS runtime
# Keep the upstream PostgreSQL/SSL/volume/entrypoint contract. Do not replace
# libxml2 with an incompatible ABI, remove XML support, or suppress its advisories.
# Supa uses JSONB memories and no vector extension; this candidate is for fresh DBs.
RUN apt-mark hold postgresql-18 postgresql-client-18 \
    && apt-get update && apt-get upgrade -y \
    && apt-get purge -y --no-auto-remove postgresql-18-pgvector \
    && test "$(dpkg-query -W -f='${Status}' postgresql-18)" = 'install ok installed' \
    && apt-mark unhold postgresql-18 postgresql-client-18 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=gosu-build --chmod=755 /out/gosu /usr/local/bin/gosu
RUN gosu --version && test "$(gosu postgres id -u)" = "$(id -u postgres)"
