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
    assert.equal(resource.variables.SUPA_ALLOW_OPENAI.value, 'false');
    assert.equal(resource.variables.SUPA_CONTROLLED_INFERENCE.value, 'false');
  }
  assert.equal(api.variables.SUPA_API_SCHEDULER_ENABLED.value, 'false');
  assert.equal(api.variables.SUPA_WALLET_ENABLED.value, 'false');
});
test('credentials and exact origin are references, never embedded or read from local evaluation files', () => {
  for(const name of ['SUPA_SITE_ORIGIN','SUPA_STAGING_USERNAME','SUPA_STAGING_PASSWORD','SUPA_WEB_OPERATORS','SUPA_API_TOKEN']) {
    assert.deepEqual(frontend.variables[name], {type:'sharedReference',name});
  }
  assert.deepEqual(api.variables.SUPA_API_TOKEN,frontend.variables.SUPA_API_TOKEN);
  assert.deepEqual(api.variables.SUPA_OPERATOR_CREDENTIALS, {type:'sharedReference',name:'SUPA_OPERATOR_CREDENTIALS'});
  assert.ok(!frontend.variables.SUPA_OPERATOR_CREDENTIALS);
  assert.ok(!frontend.variables.SUPA_WEB_USERNAME);
  assert.ok(!api.variables.SUPA_OPERATOR_ID);
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
  const credential={id:'712b9168-7abd-49ce-8301-bfe1d2babd31',role:'viewer',token:'o'.repeat(32)};
  const account={username:'operator',password:'p'.repeat(24),api_token:credential.token};
  const fixtures={SUPA_SITE_ORIGIN:'https://staging.example.test',SUPA_STAGING_USERNAME:'viewer',SUPA_STAGING_PASSWORD:'v'.repeat(24),SUPA_API_TOKEN:'t'.repeat(32),SUPA_OPERATOR_CREDENTIALS:JSON.stringify([credential]),SUPA_WEB_OPERATORS:JSON.stringify([account])};
  assert.equal(preflight(fixtures).ready,true);
  for(const extra of [
    {SUPA_WEB_USERNAME:'legacy'},
    {SUPA_OPERATOR_ID:credential.id},
    {SUPA_OPERATOR_CREDENTIALS:JSON.stringify([{...credential,token:fixtures.SUPA_API_TOKEN}])},
    {SUPA_OPERATOR_CREDENTIALS:JSON.stringify([credential,credential])},
    {SUPA_OPERATOR_CREDENTIALS:JSON.stringify([{...credential,role:'admin'}])},
    {SUPA_WEB_OPERATORS:JSON.stringify([{...account,api_token:'x'.repeat(32)}])},
    {SUPA_WEB_OPERATORS:JSON.stringify([{...account,username:fixtures.SUPA_STAGING_USERNAME}])},
    {SUPA_WEB_OPERATORS:JSON.stringify([{...account,password:fixtures.SUPA_STAGING_PASSWORD}])},
    {SUPA_WEB_OPERATORS:'[]'}, {SUPA_OPERATOR_CREDENTIALS:'null'},
  ]) assert.equal(preflight({...fixtures,...extra}).ready,false);
  assert.equal(preflight({...fixtures,SUPA_API_TOKEN:'t'.repeat(257)}).ready,false);
  const rejected=preflight({...fixtures,SUPA_SITE_ORIGIN:'http://bad.example.test'});
  assert.equal(rejected.ready,false);
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_API_TOKEN));
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_STAGING_PASSWORD));
});

test('database major and persistent mount match the reviewed recovery target', () => {
  const database = resources.find(resource => resource.type === 'database');
  assert.equal(database.image, 'ghcr.io/railwayapp-templates/postgres-ssl:18');
  assert.equal(database.defaultMountPath, '/var/lib/postgresql/data');
});

test('container runtime entrypoints preserve health ports, non-root users and SIGTERM delivery', async () => {
  const { readFile } = await import('node:fs/promises');
  for (const [resource, port, user] of [[api, '8080', '10001'], [worker, '8081', '10001'], [frontend, '3000', 'node']]) {
    const dockerfile = await readFile(new URL('../' + resource.build.dockerfilePath, import.meta.url), 'utf8');
    const runtime = dockerfile.split(/FROM .+ AS runtime\n/)[1];
    assert.ok(runtime, 'a separate runtime stage is required');
    assert.match(runtime, new RegExp(`^USER ${user}$`, 'm'));
    assert.match(runtime, new RegExp(`^EXPOSE ${port}$`, 'm'));
    assert.equal(resource.variables.PORT.value, port);
    if (resource === frontend) {
      assert.match(runtime, /exec node node_modules\/next\/dist\/bin\/next start --hostname :: --port/);
    } else {
      assert.match(runtime, /^ENTRYPOINT \["\/usr\/local\/bin\/supa-(api|runtime)"\]$/m);
    }
    assert.doesNotMatch(runtime, /^(?:ARG|ENV) (?:OPENAI_API_KEY|SUPA_OPERATOR_CREDENTIALS|SUPA_API_TOKEN)=/m);
    assert.doesNotMatch(dockerfile, /^COPY \.\s/m, 'never import the whole checkout into an image');
  }
});
