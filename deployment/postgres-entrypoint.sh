#!/usr/bin/env bash
# Fresh isolated PostgreSQL 18 candidate. No backup keys or cloud access.
set -euo pipefail
umask 077
if [[ ${1:-} == -* ]]; then set -- postgres "$@"; fi
if [[ ${1:-} != postgres ]]; then exec "$@"; fi
PGROOT=/var/lib/postgresql/data
: "${PGDATA:=$PGROOT/pgdata}"
export PGDATA
component=${PGDATA#"$PGROOT"/}
[[ "$PGDATA" == "$PGROOT/$component" && "$component" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'unexpected_pgdata' >&2; exit 1; }
TLS_DIR="$PGROOT/supa-tls"
for path in "$PGROOT" "$PGDATA" "$TLS_DIR"; do
    [[ ! -L "$path" ]] || { echo 'symlink_database_path_rejected' >&2; exit 1; }
done
if [[ -f "$PGDATA/PG_VERSION" ]]; then
    [[ $(cat "$PGDATA/PG_VERSION") == 18 ]] || { echo 'postgres_major_mismatch' >&2; exit 1; }
elif [[ -d "$PGDATA" ]]; then
    existing=$(find "$PGDATA" -mindepth 1 -maxdepth 1 -print -quit)
    [[ -z "$existing" ]] || { echo 'unrecognized_nonempty_pgdata' >&2; exit 1; }
fi
if [[ $(id -u) == 0 ]]; then
    mkdir -p "$PGROOT" "$PGDATA" "$TLS_DIR"
    chmod 700 "$PGROOT" "$PGDATA" "$TLS_DIR"
    chown postgres:postgres "$PGROOT" "$PGDATA" "$TLS_DIR"
    exec gosu postgres "$0" "$@"
fi
mkdir -p "$PGDATA" "$TLS_DIR"
chmod 700 "$PGDATA" "$TLS_DIR"
key="$TLS_DIR/server.key"
cert="$TLS_DIR/server.crt"
[[ ! -L "$key" && ! -L "$cert" ]] || { echo 'symlink_tls_file_rejected' >&2; exit 1; }
# Partial enrollment is an operator-recovery boundary, not permission to overwrite.
if { [[ -e "$key" ]] && [[ ! -e "$cert" ]]; } || { [[ ! -e "$key" ]] && [[ -e "$cert" ]]; }; then
    echo 'incomplete_tls_pair_requires_review' >&2; exit 1
fi
pending=$(mktemp -d "$TLS_DIR/.pending-XXXXXX")
trap 'rm -rf -- "$pending"' EXIT
subject=/CN=supa-postgres
sans=subjectAltName=DNS:supa-postgres,DNS:supa-postgres.railway.internal,DNS:localhost,IP:127.0.0.1
if [[ ! -e "$key" ]]; then
    openssl req -new -x509 -newkey rsa:3072 -nodes -days 90 -subj "$subject" -addext "$sans" \
        -keyout "$pending/server.key" -out "$pending/server.crt" >/dev/null 2>&1
    chmod 600 "$pending/server.key" "$pending/server.crt"
    mv "$pending/server.key" "$key"
    mv "$pending/server.crt" "$cert"
else
    [[ $(stat -c %a "$key") == 600 && $(stat -c %u "$key") == "$(id -u)" ]] || { echo 'private_tls_key_permissions_required' >&2; exit 1; }
    openssl pkey -in "$key" -pubout -outform DER > "$pending/key.pub" 2>/dev/null
    openssl x509 -in "$cert" -pubkey -noout | openssl pkey -pubin -outform DER > "$pending/cert.pub" 2>/dev/null
    cmp -s "$pending/key.pub" "$pending/cert.pub" || { echo 'tls_identity_mismatch' >&2; exit 1; }
    if ! openssl x509 -in "$cert" -checkend 2592000 -noout >/dev/null 2>&1; then
        # Preserve the private key when renewing the self-signed certificate.
        openssl req -new -x509 -key "$key" -days 90 -subj "$subject" -addext "$sans" \
            -out "$pending/server.crt" >/dev/null 2>&1
        chmod 600 "$pending/server.crt"
        mv "$pending/server.crt" "$cert"
    fi
fi
rm -rf -- "$pending"
trap - EXIT
# TLS also applies to the temporary initdb server. Certificates live beside PGDATA
# so initdb still receives an empty directory. No credentials enter argv or logs.
exec /usr/libexec/postgresql18/docker-entrypoint.sh "$@" --port=5432 -c ssl=on \
    -c "ssl_cert_file=$cert" -c "ssl_key_file=$key"
