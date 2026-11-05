# syntax=docker/dockerfile:1
# Opt-in Linux/amd64 validation candidate, not the active deployment recipe.
# Build against frozen app bfc29f500af08e0e2cc4676b8950b5b17f26007a.
# The Debian/custom-package scan workflow cannot validate this candidate unchanged.
FROM node:26.13.0-alpine3.23@sha256:cda001211d4b778be8c44dce67cd65ae14619baba11b9217da91bc81d337c91d AS build
# Both stages use musl; never copy Debian native modules into Alpine.
RUN apk add --no-cache zlib=1.3.2-r1
ENV NEXT_TELEMETRY_DISABLED=1 SUPA_SITE_MODE=staging
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/next.config.ts web/next-env.d.ts web/tsconfig.json web/postcss.config.mjs ./
COPY web/src ./src
COPY web/public ./public
COPY reference-web/index.html reference-web/not-found.html reference-web/style.css reference-web/app.js reference-web/scene.js reference-web/wheel.js reference-web/platform.js reference-web/backend.mjs reference-web/launch.mjs reference-web/backend.d.mts /app/reference-web/
COPY reference-web/public /app/reference-web/public
RUN npm run build && npm prune --omit=dev

FROM node:26.13.0-alpine3.23@sha256:cda001211d4b778be8c44dce67cd65ae14619baba11b9217da91bc81d337c91d AS runtime
# Candidate only: vendor-signed Alpine packages; preserve visible package inventory.
# No publication until all-severity scans and actual runtime checks pass.
RUN apk add --no-cache zlib=1.3.2-r1 ca-certificates \
    && apk info -v | grep -qx 'zlib-1\.3\.2-r1' \
    && addgroup -S -g 65532 supa \
    && adduser -S -D -H -u 65532 -G supa supa \
    && mkdir -p /nodejs/bin \
    && ln -s /usr/local/bin/node /nodejs/bin/node \
    && rm -rf /usr/local/lib/node_modules/npm /opt/yarn-* \
    && rm -f /usr/local/bin/npm /usr/local/bin/npx /usr/local/bin/yarn /usr/local/bin/yarnpkg
# Node 26 LTS includes patched SQLite; preserve the approved Next.js app.
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1 SUPA_SITE_MODE=staging PORT=3000
WORKDIR /app/web
COPY --from=build --chown=65532:65532 /app/web/package.json ./package.json
COPY --from=build --chown=65532:65532 /app/web/node_modules ./node_modules
COPY --from=build --chown=65532:65532 /app/web/.next ./.next
COPY --from=build --chown=65532:65532 /app/web/public ./public
COPY --from=build --chown=65532:65532 /app/web/next.config.ts ./next.config.ts
# The approved experience is served verbatim; contracts/auth stay server-side.
COPY --from=build --chown=65532:65532 /app/web/src/lib ./src/lib
COPY --from=build --chown=65532:65532 /app/reference-web /app/reference-web
USER 65532:65532
EXPOSE 3000
ENTRYPOINT ["/nodejs/bin/node"]
# Next.js reads PORT from the environment; no shell expansion or wrapper process.
CMD ["node_modules/next/dist/bin/next", "start", "--hostname", "0.0.0.0"]
