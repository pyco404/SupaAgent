import { defineRailway, github, postgres, project, service } from "railway/iac";

// Authoring only. Evaluation constructs a graph; it makes no network requests.
export default defineRailway((ctx) => {
  if (!ctx.isEnvironment("staging")) {
    throw new Error("Supa infrastructure is restricted to the staging environment");
  }
  const db = postgres("supa-staging-postgres");
  // No generated/custom domains or public TCP proxy, including for PostgreSQL.
  db.networking = { serviceDomains: {}, customDomains: {}, tcpProxies: {} };
  const source = github("pyco404/SupaAgent", { branch: "main", rootDirectory: "/" });
  const deploy = { restartPolicyType: "ON_FAILURE" as const, restartPolicyMaxRetries: 3 };
  const backend = {
    DATABASE_URL: db.env.DATABASE_URL,
    SUPA_PRIVATE_STAGING: "true",
    SUPA_PROVIDER: "offline",
  };
  const api = service("supa-api", {
    source,
    build: { builder: "DOCKERFILE", dockerfilePath: "deployment/api.Dockerfile" },
    deploy,
    healthcheck: "/ready", healthcheckTimeout: 120, replicas: 1,
    domains: [], tcp: [],
    env: {
      ...backend,
      PORT: "8080", SUPA_BIND: "[::]:8080",
      SUPA_API_TOKEN: ctx.shared.SUPA_API_TOKEN,
      SUPA_API_SCHEDULER_ENABLED: "false",
      SUPA_WALLET_ENABLED: "false",
    },
  });
  const worker = service("supa-worker", {
    source,
    build: { builder: "DOCKERFILE", dockerfilePath: "deployment/worker.Dockerfile" },
    deploy,
    healthcheck: "/ready", healthcheckTimeout: 120, replicas: 1,
    domains: [], tcp: [],
    env: {
      ...backend,
      PORT: "8081", SUPA_WORKER_BIND: "[::]:8081",
      SUPA_WORKER_ENABLED: "false", SUPA_WORKER_POLL_SECONDS: "5",
    },
  });
  const frontend = service("supa-frontend", {
    source,
    build: { builder: "DOCKERFILE", dockerfilePath: "deployment/frontend.Dockerfile" },
    deploy,
    healthcheck: "/healthz", healthcheckTimeout: 120, replicas: 1,
    // Add a reviewed domain only after its exact HTTPS origin is configured.
    domains: [], tcp: [],
    env: {
      PORT: "3000", SUPA_SITE_MODE: "staging",
      SUPA_SITE_ORIGIN: ctx.shared.SUPA_SITE_ORIGIN,
      SUPA_STAGING_USERNAME: ctx.shared.SUPA_STAGING_USERNAME,
      SUPA_STAGING_PASSWORD: ctx.shared.SUPA_STAGING_PASSWORD,
      SUPA_WEB_USERNAME: ctx.shared.SUPA_WEB_USERNAME,
      SUPA_WEB_PASSWORD: ctx.shared.SUPA_WEB_PASSWORD,
      SUPA_API_TOKEN: ctx.shared.SUPA_API_TOKEN,
      SUPA_BACKEND_URL: "http://supa-api.railway.internal:8080",
    },
  });
  return project("Supa Agent Staging", { resources: [db, api, worker, frontend] });
});
