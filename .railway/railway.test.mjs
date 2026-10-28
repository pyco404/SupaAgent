import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { createRailwayContext, project, validateGraph, RAILWAY_GRAPH_VERSION } from 'railway/iac';
import program from './railway.ts';
import { preflight } from './preflight.mjs';

const definition = await program(createRailwayContext({ command:'plan', environment:'staging' }), project);
const resources = definition.resources;
const api = resources.find(r=>r.name === 'supa-api');
const worker = resources.find(r=>r.name === 'supa-worker');
const frontend = resources.find(r=>r.name === 'supa-frontend');

test('official SDK validates four unique staging resources with a fresh PostgreSQL helper', () => {
  assert.equal(resources.length, 4);
  assert.equal(resources.find(r=>r.type === 'database').engine, 'postgres');
  assert.deepEqual(validateGraph({ version:RAILWAY_GRAPH_VERSION, resources, edges:[] }), []);
});
test('all application source, Dockerfiles, ports and private network boundaries are declared', () => {
  for (const resource of [api,worker,frontend]) {
    assert.equal(resource.source.repo, 'pyco404/SupaAgent');
    assert.equal(resource.source.branch, 'main');
    assert.equal(resource.build.builder, 'DOCKERFILE');
    assert.ok(existsSync(new URL('../'+resource.build.dockerfilePath, import.meta.url)));
    assert.equal(resource.deploy.numReplicas, 1);
    assert.ok(resource.deploy.healthcheckPath);
  }
  for (const resource of resources) {
    assert.equal(Object.keys(resource.networking?.customDomains ?? {}).length,0);
    assert.equal(Object.keys(resource.networking?.serviceDomains ?? {}).length,0);
    assert.equal(Object.keys(resource.networking?.tcpProxies ?? {}).length,0);
  }
  assert.equal(api.variables.PORT.value, '8080');
  assert.equal(worker.variables.PORT.value, '8081');
});
test('worker remains disabled and offline; API scheduler and wallets remain disabled', () => {
  assert.equal(worker.variables.SUPA_WORKER_ENABLED.value, 'false');
  for(const resource of [api,worker]) {
    assert.equal(resource.variables.SUPA_PROVIDER.value, 'offline');
    assert.equal(resource.variables.DATABASE_URL.type, 'reference');
    assert.equal(resource.variables.DATABASE_URL.resource, 'database.supa-staging-postgres');
  }
  assert.equal(api.variables.SUPA_API_SCHEDULER_ENABLED.value, 'false');
  assert.equal(api.variables.SUPA_WALLET_ENABLED.value, 'false');
});
test('credentials and exact origin are references, never embedded or read from local evaluation files', () => {
  for(const name of ['SUPA_SITE_ORIGIN','SUPA_STAGING_USERNAME','SUPA_STAGING_PASSWORD','SUPA_WEB_USERNAME','SUPA_WEB_PASSWORD','SUPA_API_TOKEN']) {
    assert.deepEqual(frontend.variables[name], {type:'sharedReference',name});
  }
  assert.deepEqual(api.variables.SUPA_API_TOKEN,frontend.variables.SUPA_API_TOKEN);
  for(const resource of resources) {
    assert.ok(!resource.variables?.OPENAI_API_KEY);
    assert.ok(!resource.variables?.SUPA_EVAL_DATABASE_URL);
  }
});
test('non-staging environments fail before defining any resource', async () => {
  for(const environment of ['production','development',undefined]) {
    await assert.rejects(async()=>program(createRailwayContext({environment}),project), /restricted to the staging/);
  }
});
test('local preflight fails clearly without secrets and never outputs provided sensitive values', () => {
  const missing=preflight({});
  assert.equal(missing.ready,false);
  assert.equal(missing.failures.length,6);
  assert.equal(missing.network_requests,0);
  const fixtures={SUPA_SITE_ORIGIN:'https://staging.example.test',SUPA_STAGING_USERNAME:'viewer',SUPA_WEB_USERNAME:'operator',SUPA_STAGING_PASSWORD:'v'.repeat(24),SUPA_WEB_PASSWORD:'o'.repeat(24),SUPA_API_TOKEN:'t'.repeat(32)};
  assert.equal(preflight(fixtures).ready,true);
  assert.equal(preflight({...fixtures,SUPA_WEB_USERNAME:fixtures.SUPA_STAGING_USERNAME}).ready,false);
  assert.equal(preflight({...fixtures,SUPA_API_TOKEN:'t'.repeat(257)}).ready,false);
  const rejected=preflight({...fixtures,SUPA_SITE_ORIGIN:'http://bad.example.test',SUPA_WEB_PASSWORD:fixtures.SUPA_STAGING_PASSWORD});
  assert.equal(rejected.ready,false);
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_API_TOKEN));
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_STAGING_PASSWORD));
});
