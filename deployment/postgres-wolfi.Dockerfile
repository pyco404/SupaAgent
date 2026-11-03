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

FROM cgr.dev/chainguard/wolfi-base:latest@sha256:629d9acaceda7746cd9293365eb0d8e8fabf74e91830b05470cfbee20b53f447 AS runtime
# Fresh staging candidate only. No existing primary volume, ABI replacement or
# shared-library patches. APK verifies the vendor's signed packages by default.
# Match the base's glibc family; pin LLVM's ABI-16 build instead of the legacy
# virtual provider that would pull vulnerable libxml2.so.2 back into the image.
# -locale-en contains generated en_US.utf8; -locales does not ship that data.
RUN apk add --no-cache \
      postgresql-18-base=18.6-r7 postgresql-18-client-base=18.6-r7 \
      postgresql-18-contrib-base=18.6-r7 postgresql-18-oci-entrypoint-base=18.6-r7 \
      libxml2-16=2.15.4-r2 openssl-4.0=4.0.3-r8 bash=5.3-r14 \
      glibc-2.44=2.44-r13 glibc-2.44-locale-en=2.44-r13 \
      posix-libc-utils-2.44=2.44-r13 libLLVM-19=19.1.7-r31
ENV PATH=/usr/libexec/postgresql18:/usr/local/bin:/usr/bin:/bin \
    PG_MAJOR=18 PG_VERSION=18.6 LANG=en_US.utf8 PGDATA=/var/lib/postgresql/data/pgdata
RUN set -eu; \
    if ! id postgres >/dev/null 2>&1; then \
      addgroup -S -g 70 postgres; \
      adduser -S -D -H -u 70 -G postgres -h /var/lib/postgresql postgres; \
    fi; \
    mkdir -p /var/lib/postgresql/data /var/lib/postgres/initdb /var/run/postgresql; \
    chown postgres:postgres /var/lib/postgresql/data /var/run/postgresql; \
    chmod 3777 /var/run/postgresql; \
    apk info -v libxml2-16 libLLVM-19 glibc-2.44 glibc-2.44-locale-en posix-libc-utils-2.44 postgresql-18-base openssl-4.0; \
    locale -a > /tmp/postgres-locales; \
    grep -Fx 'en_US.utf8' /tmp/postgres-locales; \
    test "$(LC_ALL=en_US.utf8 locale charmap)" = UTF-8; \
    rm /tmp/postgres-locales; \
    postgres --version; \
    ldd /usr/libexec/postgresql18/postgres > /tmp/postgres-libraries 2>&1; \
    cat /tmp/postgres-libraries; \
    ! grep -E 'not found|Error loading' /tmp/postgres-libraries; \
    test -z "$(find /usr/lib -name 'libxml2.so.2*' -print -quit)"; \
    rm /tmp/postgres-libraries
COPY --from=gosu-build --chmod=755 /out/gosu /usr/local/bin/gosu
RUN gosu --version && test "$(gosu postgres id -u)" = "$(id -u postgres)"
COPY --chmod=755 deployment/postgres-entrypoint.sh /usr/local/bin/supa-postgres-entrypoint
VOLUME ["/var/lib/postgresql/data"]
EXPOSE 5432
STOPSIGNAL SIGINT
ENTRYPOINT ["/usr/local/bin/supa-postgres-entrypoint"]
CMD ["postgres"]
