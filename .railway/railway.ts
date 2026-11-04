import { readFileSync } from "node:fs";
import { bucket, database, defineRailway, github, project, service, volume, type RailwayContext } from "railway/iac";

export const stagingSession = Object.freeze({
  projectId: "99648d2a-69a1-4d8c-98d0-7f2a26543c9c",
  environmentId: "ff0b2092-e42b-4a89-ae25-441ab95d92d8",
  operationMode: "normal-staging",
  monthlyUsagePlanningUSD: { minimum: 10, maximum: 20 },
  deadlineAutomationRequired: false,
  syntheticDataOnly: true,
  proposedVolumeSizeMB: 1000,
  customReplicaLimitsVerified: false,
  applicationRevision: "bfc29f500af08e0e2cc4676b8950b5b17f26007a",
  workerReplicas: 0,
  // Planning only: no resource creation, billing-limit change or runtime authorization.
  deploymentAuthorized: false,
});

export const verifiedPostgresReference = "ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8";

export function reviewedPostgresImage(value: unknown): string {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("reviewed private PostgreSQL image required");
  }
  const record = value as Record<string, unknown>;
  const reference = record.reference;
  if (record.approved !== true || record.private !== true ||
      typeof reference !== "string" ||
      reference !== verifiedPostgresReference ||
      /@sha256:0{64}$/.test(reference) ||
      typeof record.source_revision !== "string" || !/^[a-f0-9]{40}$/.test(record.source_revision) ||
      /^0{40}$/.test(record.source_revision) ||
      typeof record.scan_run_url !== "string" ||
      !/^https:\/\/github\.com\/pyco404\/SupaAgent\/actions\/runs\/[1-9][0-9]*$/.test(record.scan_run_url) ||
      record.high_critical_findings !== 0 || record.secret_findings !== 0 ||
      record.integrity_verified !== true || record.anonymous_access_denied !== true ||
      record.package_repository !== "pyco404/supa-agent-infra" ||
      record.configuration_digest !== "sha256:0228d9646b8387201f20918ea903474f1b283d43c9a049e29ccda88701771abd" ||
      record.verified_layers !== 16 || record.platform !== "linux/amd64" ||
      typeof record.verification_evidence_sha256 !== "string" ||
      !/^[a-f0-9]{64}$/.test(record.verification_evidence_sha256) ||
      record.deployment_authorized !== false) {
    throw new Error("reviewed private PostgreSQL digest and clean scan evidence required");
  }
  return reference;
}

// Authoring only. Evaluation constructs a graph; it makes no network requests.
export function stagingDefinition(ctx: RailwayContext, imageApproval: unknown) {
  if (!ctx.isEnvironment("staging")) {
    throw new Error("Supa infrastructure is restricted to the staging environment");
  }
  if (ctx.projectId !== stagingSession.projectId || ctx.environmentId !== stagingSession.environmentId) {
    throw new Error("exact existing Supa Agent project and staging environment required");
  }
  const region = "europe-west4-drams3a";
  const db = database("supa-postgres", "postgres", {
    image: reviewedPostgresImage(imageApproval),
    region, output: "DATABASE_URL", defaultMountPath: "/var/lib/postgresql/data",
  });
  const storage = volume("supa-postgres-data", { sizeMB: stagingSession.proposedVolumeSizeMB, region });
  // Whole-project plans must retain this existing independent-backup resource.
  const backups = bucket("supa-agent-backups", { region: "ams" });
  db.volumeAttachments = { data: { volume: storage.address,
    mountPath: "/var/lib/postgresql/data", backupSchedules: ["DAILY"], volumeConfig: storage.config } };
  db.deploy = { ...db.deploy, restartPolicyType: "ON_FAILURE", restartPolicyMaxRetries: 3,
    drainingSeconds: 120, requiredMountPath: "/var/lib/postgresql/data" };
  // No generated/custom domains or public TCP proxy, including for PostgreSQL.
  db.networking = { serviceDomains: {}, customDomains: {}, tcpProxies: {} };
  // The source SHA is frozen; verify the deployed SHA and disable automatic deploys
  // separately in Railway before enabling the services.
  const source = github("pyco404/SupaAgent", { branch: "main",
    commitSha: stagingSession.applicationRevision, rootDirectory: "/" });
  const deploy = { restartPolicyType: "ON_FAILURE" as const, restartPolicyMaxRetries: 3, region, drainingSeconds: 60 };
  const backend = {
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
      // Separately provisioned restricted login, never the database helper/admin URL.
      DATABASE_URL: ctx.shared.SUPA_API_DATABASE_URL,
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
    deploy: { ...deploy, multiRegionConfig: { [region]: { numReplicas: stagingSession.workerReplicas } } },
    healthcheck: "/ready", healthcheckTimeout: 120, replicas: stagingSession.workerReplicas,
    domains: [], tcp: [],
    env: {
      ...backend,
      DATABASE_URL: ctx.shared.SUPA_WORKER_DATABASE_URL,
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
  return project("Supa Agent", { resources: [db, api, worker, frontend, storage, backups] });
}

export default defineRailway((ctx) => stagingDefinition(ctx, JSON.parse(
  readFileSync(new URL("../config/railway-staging-image.json", import.meta.url), "utf8"),
)));
