# syntax=docker/dockerfile:1
FROM node:26.13.0-trixie-slim@sha256:e4db4c914bf8ad241bd24804fc25d272d89bad97e68cdc8a1414862790a02c39 AS build
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

FROM gcr.io/distroless/nodejs26-debian13:nonroot@sha256:9b945e92c1b5463a4b75f11e8417515e4bea7355f6adfd79ec220fbbe905dda8 AS runtime
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
