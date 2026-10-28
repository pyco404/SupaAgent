# syntax=docker/dockerfile:1
FROM node:24-bookworm-slim AS build
ENV NEXT_TELEMETRY_DISABLED=1 SUPA_SITE_MODE=staging
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/next.config.ts web/next-env.d.ts web/tsconfig.json web/postcss.config.mjs ./
COPY web/src ./src
COPY web/public ./public
COPY reference-web/index.html reference-web/style.css reference-web/app.js reference-web/scene.js reference-web/wheel.js reference-web/platform.js reference-web/backend.mjs reference-web/backend.d.mts /app/reference-web/
COPY reference-web/public /app/reference-web/public
RUN npm run build && npm prune --omit=dev

FROM node:24-bookworm-slim AS runtime
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1 SUPA_SITE_MODE=staging PORT=3000
WORKDIR /app/web
COPY --from=build --chown=node:node /app/web/package.json ./package.json
COPY --from=build --chown=node:node /app/web/node_modules ./node_modules
COPY --from=build --chown=node:node /app/web/.next ./.next
COPY --from=build --chown=node:node /app/web/public ./public
COPY --from=build --chown=node:node /app/web/next.config.ts ./next.config.ts
# The approved experience is served verbatim; contracts/auth stay server-side.
COPY --from=build --chown=node:node /app/web/src/lib ./src/lib
COPY --from=build --chown=node:node /app/reference-web /app/reference-web
USER node
EXPOSE 3000
CMD ["sh", "-c", "exec node node_modules/next/dist/bin/next start --hostname :: --port \"$PORT\""]
