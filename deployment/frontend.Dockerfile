# syntax=docker/dockerfile:1
FROM node:24-trixie-slim AS build
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

FROM gcr.io/distroless/nodejs24-debian13:nonroot@sha256:b8cec4b2f2126ff7bd034ec9ac207ffb3272667dfdc48376e3b035d3270549ba AS runtime
# Preserve Node 24 and the approved Next.js app without a shell/package manager.
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
