import { defineRailway, github, postgres, project, service } from "railway/iac";

// Authoring only. Evaluation constructs a graph; it makes no network requests.
export default defineRailway((ctx) => {
  if (!ctx.isEnvironment("staging")) {
    throw new Error("Supa infrastructure is restricted to the staging environment");
  }
  const region = "europe-west4-drams3a";
  const db = postgres("supa-postgres", { region });
  // No generated/custom domains or public TCP proxy, including for PostgreSQL.
  db.networking = { serviceDomains: {}, customDomains: {}, tcpProxies: {} };
  const source = github("pyco404/SupaAgent", { branch: "main", rootDirectory: "/" });
  const deploy = { restartPolicyType: "ON_FAILURE" as const, restartPolicyMaxRetries: 3, region };
  const backend = {
    DATABASE_URL: db.env.DATABASE_URL,
    SUPA_PRIVATE_STAGING: "true",
    SUPA_PROVIDER: "offline",
    SUPA_ALLOW_OPENAI: "false",
    SUPA_ALLOW_ANTHROPIC: "false",
    SUPA_CONTROLLED_INFERENCE: "false",
    SUPA_CONTROLLED_WORK: "false",
    SUPA_WALLET_ENABLED: "false",
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
      SUPA_OPERATOR_CREDENTIALS: ctx.shared.SUPA_OPERATOR_CREDENTIALS,
      SUPA_OPERATOR_SESSION_DEADLINES: ctx.shared.SUPA_OPERATOR_SESSION_DEADLINES,
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
  const frontend = service("supa-web", {
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
      SUPA_WEB_OPERATORS: ctx.shared.SUPA_WEB_OPERATORS,
      SUPA_API_TOKEN: ctx.shared.SUPA_API_TOKEN,
      SUPA_BACKEND_URL: "http://supa-api.railway.internal:8080",
    },
  });
  // Authoring label only: bind to the existing project/environment IDs before
  // an approved plan. Never apply with an unreviewed delete/resource inventory.
  return project("Supa Agent", { resources: [db, api, worker, frontend] });
});
