#!/usr/bin/env bash
# Single-use, owner-authorized repair for the empty Railway staging cluster only.
# Run as postgres inside the existing service. No private material leaves its volume.
set -euo pipefail
umask 077
[[ ${1:-} == --authorize-staging-tls-repair && $# == 1 ]]
[[ ${RAILWAY_PROJECT_ID:-} == 99648d2a-69a1-4d8c-98d0-7f2a26543c9c ]]
[[ ${RAILWAY_ENVIRONMENT_ID:-} == ff0b2092-e42b-4a89-ae25-441ab95d92d8 ]]
[[ ${RAILWAY_SERVICE_ID:-} == 77352ca9-435e-4870-b092-58365310c0f3 ]]
[[ ${POSTGRES_DB:-} == supa_staging && ${POSTGRES_USER:-} == supa_staging_admin ]]
[[ ${PGDATA:-} == /var/lib/postgresql/data/pgdata && $(id -u) == "$(id -u postgres)" ]]
for tool in openssl psql pg_ctl pg_isready install sed stat sha256sum gosu; do command -v "$tool" >/dev/null; done
root=/var/lib/postgresql/data
host=supa-postgres.railway.internal
tls="$root/supa-tls"
for path in "$root" "$PGDATA" "$tls" "$tls/server.crt" "$tls/server.key" "$PGDATA/pg_hba.conf" "$PGDATA/postgresql.auto.conf"; do
    [[ ! -L "$path" && -e "$path" ]]
done
[[ ! -e "$tls/ca.crt" ]]
export PGPASSWORD="$POSTGRES_PASSWORD" PGSSLMODE=verify-full PGSSLROOTCERT="$tls/server.crt"
export PGOPTIONS='-c default_transaction_read_only=on -c statement_timeout=10000'
query() { psql -XqAt -v ON_ERROR_STOP=1 -h "$host" -U "$POSTGRES_USER" -d "$POSTGRES_DB" "$@"; }
[[ $(query -c "SELECT current_database()||':'||current_user||':'||system_identifier::text FROM pg_control_system()") == supa_staging:supa_staging_admin:7704432314739101731 ]]
[[ $(query -c "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema')") == 0 ]]
[[ $(query -c 'SELECT count(*) FROM pg_roles WHERE rolcanlogin') == 1 ]]
[[ $(query -c 'SHOW ssl_cert_file') == "$tls/server.crt" && $(query -c 'SHOW ssl_key_file') == "$tls/server.key" ]]
work=$(mktemp -d "$root/supa-tls-repair-XXXXXXXX")
mkdir "$work/original" "$work/candidate"
chmod 700 "$work" "$work/original" "$work/candidate"
cp -p "$tls/server.crt" "$tls/server.key" "$PGDATA/pg_hba.conf" "$PGDATA/postgresql.auto.conf" "$work/original/"
( cd "$work/original"; sha256sum server.crt server.key pg_hba.conf postgresql.auto.conf > SHA256SUMS )
installed=0
rollback() {
    code=$?
    trap - ERR
    if [[ "$installed" == 1 ]]; then
        # One recovery attempt only. Keep both old and candidate identities for review.
        cp -p "$work/original/server.crt" "$tls/server.crt"
        cp -p "$work/original/server.key" "$tls/server.key"
        cp -p "$work/original/pg_hba.conf" "$PGDATA/pg_hba.conf"
        cp -p "$work/original/postgresql.auto.conf" "$PGDATA/postgresql.auto.conf"
        cp -p "$work/original/server.crt" "$tls/ca.crt"
        pg_ctl -D "$PGDATA" reload > "$work/rollback-reload.log" 2>&1
        sleep 1
        export PGSSLROOTCERT="$work/original/server.crt"
        query -c 'SELECT 1' > "$work/rollback-readiness.log" 2>&1
        printf '%s\n' 'TLS_REPAIR_FAILED_ORIGINAL_CONFIGURATION_RESTORED' >&2
    else
        printf '%s\n' 'TLS_REPAIR_FAILED_BEFORE_INSTALLATION' >&2
    fi
    printf 'preserved_repair_directory=%s\n' "$work" >&2
    exit "$code"
}
trap rollback ERR
cd "$work/candidate"
openssl req -new -x509 -newkey rsa:3072 -noenc -sha256 -days 365 \
    -subj '/CN=Supa Agent Staging PostgreSQL CA' \
    -addext 'basicConstraints=critical,CA:TRUE,pathlen:0' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' \
    -keyout ca.key -out ca.crt > "$work/ca-generation.log" 2>&1
openssl req -new -newkey rsa:3072 -noenc -sha256 -subj "/CN=$host" \
    -keyout server.key -out server.csr > "$work/server-generation.log" 2>&1
cat > server.ext <<'EXT'
[server_cert]
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:supa-postgres.railway.internal,DNS:supa-postgres
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid,issuer
EXT
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
    -days 90 -sha256 -extfile server.ext -extensions server_cert -out server.crt > "$work/certificate-signing.log" 2>&1
chmod 600 ca.key ca.crt ca.srl server.key server.crt
openssl verify -x509_strict -CAfile ca.crt -purpose sslserver -verify_hostname "$host" server.crt
openssl x509 -in server.crt -noout -ext basicConstraints | grep -q 'CA:FALSE'
openssl x509 -in server.crt -noout -ext extendedKeyUsage | grep -q 'TLS Web Server Authentication'
openssl pkey -in server.key -pubout -outform DER > key.pub 2>/dev/null
openssl x509 -in server.crt -pubkey -noout | openssl pkey -pubin -outform DER > cert.pub 2>/dev/null
cmp -s key.pub cert.pub
cat server.crt ca.crt > server-chain.crt
# PostgreSQL loads the pair on reload only after both replacements are installed.
installed=1
PGOPTIONS='-c statement_timeout=10000' query -c "ALTER SYSTEM SET ssl_min_protocol_version='TLSv1.3'" > "$work/tls-minimum-setting.log"
install -m 600 server.key "$tls/server.key"
install -m 600 server-chain.crt "$tls/server.crt"
install -m 600 ca.crt "$tls/ca.crt"
# Preserve Unix-socket rules; require TLS+SCRAM for every TCP rule.
{ printf '%s\n' 'hostnossl all all all reject'; sed -E 's/^([[:space:]]*)host([[:space:]])/\1hostssl\2/' "$work/original/pg_hba.conf"; } > "$PGDATA/pg_hba.conf.pending-tls-repair"
chmod 600 "$PGDATA/pg_hba.conf.pending-tls-repair"
mv "$PGDATA/pg_hba.conf.pending-tls-repair" "$PGDATA/pg_hba.conf"
pg_ctl -D "$PGDATA" reload > "$work/reload.log" 2>&1
export PGSSLROOTCERT="$tls/ca.crt"
sleep 1
# New backends must use the new leaf and CA, never the old trust anchor.
query -c "SELECT json_build_object('database',current_database(),'login',current_user,'tls',(SELECT version FROM pg_stat_ssl WHERE pid=pg_backend_pid()),'minimum',current_setting('ssl_min_protocol_version'),'scram',current_setting('password_encryption'),'tables',(SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema')),'logins',(SELECT count(*) FROM pg_roles WHERE rolcanlogin));" > "$work/libpq-verification.json"
[[ $(query -c "SELECT current_setting('ssl_min_protocol_version')||':'||(SELECT version FROM pg_stat_ssl WHERE pid=pg_backend_pid())") == TLSv1.3:TLSv1.3 ]]
[[ $(query -c "SELECT count(*) FROM pg_hba_file_rules WHERE error IS NOT NULL OR (type LIKE 'host%' AND NOT (type='hostssl' AND auth_method='scram-sha-256') AND NOT (type='hostnossl' AND auth_method='reject'))") == 0 ]]
[[ $(query -c "SELECT count(*) FROM pg_authid WHERE rolcanlogin AND rolpassword LIKE 'SCRAM-SHA-256$%'") == 1 ]]
if PGSSLMODE=disable query -c 'SELECT 1' > "$work/non-tls-denial.log" 2>&1; then false; fi
if PGHOST=wrong-hostname.invalid PGHOSTADDR=127.0.0.1 psql -XqAt -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c 'SELECT 1' > "$work/hostname-denial.log" 2>&1; then false; fi
grep -q 'does not match host name' "$work/hostname-denial.log"
if PGSSLMAXPROTOCOLVERSION=TLSv1.2 query -c 'SELECT 1' > "$work/tls12-denial.log" 2>&1; then false; fi
[[ $(stat -c '%a:%u' "$tls/server.key") == "600:$(id -u)" ]]
( cd "$work/original"; sha256sum -c SHA256SUMS > "$work/original-integrity.log" )
pg_isready -h "$host" -p 5432 -U "$POSTGRES_USER" -d "$POSTGRES_DB"
trap - ERR
printf 'TLS_REPAIR_INSTALLED_AND_LIBPQ_VERIFIED\npreserved_repair_directory=%s\n' "$work"
cat "$work/libpq-verification.json"
openssl x509 -in "$tls/server.crt" -noout -fingerprint -sha256 -dates
openssl x509 -in "$tls/ca.crt" -noout -fingerprint -sha256 -dates
unset PGPASSWORD
