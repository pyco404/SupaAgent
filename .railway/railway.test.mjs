import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { createRailwayContext, project, validateGraph, RAILWAY_GRAPH_VERSION } from 'railway/iac';
import program, { reviewedPostgresImage, stagingDefinition, stagingSession } from './railway.ts';
import { preflight, targetPreflight } from './preflight.mjs';

// Synthetic reference only: no image is published or approved by this test.
const imageFixture = { approved:true, private:true,
  reference:'ghcr.io/pyco404/supa-agent-postgres@sha256:'+'1'.repeat(64),
  source_revision:'2'.repeat(40),
  scan_run_url:'https://github.com/pyco404/SupaAgent/actions/runs/1',
  high_critical_findings:0, secret_findings:0 };
const target = { command:'plan', environment:'staging', projectId:stagingSession.projectId,
  environmentId:stagingSession.environmentId };
const definition = stagingDefinition(createRailwayContext(target), imageFixture);
const resources = definition.resources;
const api = resources.find(r=>r.name === 'supa-api');
const worker = resources.find(r=>r.name === 'supa-worker');
const frontend = resources.find(r=>r.name === 'supa-web');

test('official SDK validates four unique resources with a digest-pinned PostgreSQL database', () => {
  assert.equal(resources.length, 4);
  assert.equal(resources.find(r=>r.type === 'database').engine, 'postgres');
  assert.deepEqual(validateGraph({ version:RAILWAY_GRAPH_VERSION, resources, edges:[] }), []);
});
test('unapproved actual configuration refuses graph generation without a generic fallback', async () => {
  await assert.rejects(async()=>program(createRailwayContext(target),project), /reviewed private PostgreSQL/);
  for (const change of [{projectId:'different'}, {environmentId:'different'}, {projectId:undefined}]) {
    assert.throws(()=>stagingDefinition(createRailwayContext({...target,...change}),imageFixture), /exact existing/);
  }
  assert.equal(stagingSession.maximumHours,6);
  assert.equal(stagingSession.incrementalBudgetUSD,1);
  assert.equal(stagingSession.syntheticDataOnly,true);
  assert.equal(stagingSession.deploymentAuthorized,false);
});
test('image review rejects tags, other repositories, missing evidence and findings', () => {
  assert.equal(reviewedPostgresImage(imageFixture),imageFixture.reference);
  for (const value of [null, [], {},
    ...[{approved:false},{private:false},{reference:'ghcr.io/railwayapp-templates/postgres-ssl:18'},
      {reference:'ghcr.io/pyco404/supa-agent-postgres:latest'},
      {reference:'ghcr.io/other/supa-agent-postgres@sha256:'+'1'.repeat(64)},
      {reference:'ghcr.io/pyco404/supa-agent-postgres@sha256:'+'0'.repeat(64)},
      {source_revision:null},{source_revision:'0'.repeat(40)},
      {scan_run_url:'https://github.com/other/repo/actions/runs/1'},
      {high_critical_findings:1},{high_critical_findings:'0'},
      {secret_findings:1}].map(change=>({...imageFixture,...change}))]) {
    assert.throws(()=>reviewedPostgresImage(value), /reviewed private PostgreSQL/);
  }
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
    assert.equal(resource.variables.DATABASE_URL.type, 'sharedReference');
    assert.equal(resource.variables.SUPA_ALLOW_OPENAI.value, 'false');
    assert.equal(resource.variables.SUPA_CONTROLLED_INFERENCE.value, 'false');
    assert.equal(resource.variables.SUPA_ALLOW_ANTHROPIC.value, 'false');
    assert.equal(resource.variables.SUPA_CONTROLLED_WORK.value, 'false');
    assert.equal(resource.variables.SUPA_WALLET_ENABLED.value, 'false');
  }
  assert.equal(api.variables.SUPA_API_SCHEDULER_ENABLED.value, 'false');
  assert.deepEqual(api.variables.DATABASE_URL,{type:'sharedReference',name:'SUPA_API_DATABASE_URL'});
  assert.deepEqual(worker.variables.DATABASE_URL,{type:'sharedReference',name:'SUPA_WORKER_DATABASE_URL'});
  assert.equal(api.variables.SUPA_WALLET_ENABLED.value, 'false');
});
test('authoring names and Amsterdam region match the dedicated staging proposal',()=>{
  assert.equal(definition.name,'Supa Agent');
  assert.deepEqual(resources.map(r=>r.name).sort(),['supa-api','supa-postgres','supa-web','supa-worker']);
  for(const resource of [api,worker,frontend])assert.equal(resource.deploy.region,'europe-west4-drams3a');
  assert.deepEqual(resources.find(r=>r.type==='database').deploy.multiRegionConfig,{'europe-west4-drams3a':{numReplicas:1}});
  assert.deepEqual(api.variables.SUPA_OPERATOR_SESSION_DEADLINES,{type:'sharedReference',name:'SUPA_OPERATOR_SESSION_DEADLINES'});
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
  assert.equal(missing.failures.length,7);
  assert.equal(missing.network_requests,0);
  const credential={id:'712b9168-7abd-49ce-8301-bfe1d2babd31',role:'viewer',token:'o'.repeat(32)};
  const account={username:'operator',password:'p'.repeat(24),api_token:credential.token};
  const fixtures={SUPA_SITE_ORIGIN:'https://staging.example.test',SUPA_STAGING_USERNAME:'viewer',SUPA_STAGING_PASSWORD:'v'.repeat(24),SUPA_API_TOKEN:'t'.repeat(32),SUPA_OPERATOR_CREDENTIALS:JSON.stringify([credential]),SUPA_WEB_OPERATORS:JSON.stringify([account]),SUPA_OPERATOR_SESSION_DEADLINES:JSON.stringify({[credential.id]:Math.floor(Date.now()/1000)+3600})};
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
    {SUPA_OPERATOR_SESSION_DEADLINES:'{}'},
    {SUPA_OPERATOR_SESSION_DEADLINES:JSON.stringify({[credential.id]:0})},
    {SUPA_OPERATOR_SESSION_DEADLINES:JSON.stringify({[credential.id]:Math.floor(Date.now()/1000)+86401})},
  ]) assert.equal(preflight({...fixtures,...extra}).ready,false);
  assert.equal(preflight({...fixtures,SUPA_API_TOKEN:'t'.repeat(257)}).ready,false);
  const rejected=preflight({...fixtures,SUPA_SITE_ORIGIN:'http://bad.example.test'});
  assert.equal(rejected.ready,false);
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_API_TOKEN));
  assert.ok(!JSON.stringify(rejected).includes(fixtures.SUPA_STAGING_PASSWORD));
});

test('offline target review rejects another workspace, environment, resources or bucket',()=>{
 const environment='ff0b2092-e42b-4a89-ae25-441ab95d92d8';
 const node={id:environment,name:'staging',canAccess:true,deletedAt:null,serviceInstances:{edges:[]},volumeInstances:{edges:[]}};
 const inventory={id:'99648d2a-69a1-4d8c-98d0-7f2a26543c9c',name:'Supa Agent',workspaceId:'e319f68a-9897-49ed-b119-743f1f846aa0',services:{edges:[]},environments:{edges:[{node}]}};
 const bucket={id:'76923b39-b3a7-4390-8a0b-783d8a8113e9',name:'supa-agent-backups',environmentId:environment,region:'ams',objects:0,storageBytes:0};
 assert.equal(targetPreflight(inventory,bucket).ready,true);
 assert.equal(targetPreflight(inventory,bucket).deployment_authorized,false);
 for(const change of [{id:'different'},{workspaceId:'different'},{services:{edges:[{node:{name:'unexpected'}}]}},{environments:{edges:[]}}])assert.equal(targetPreflight({...inventory,...change},bucket).ready,false);
 for(const change of [{id:'different'},{region:'iad'},{objects:1},{storageBytes:1},{environmentId:'production'}])assert.equal(targetPreflight(inventory,{...bucket,...change}).ready,false);
});

test('database major and persistent mount match the reviewed recovery target', () => {
  const database = resources.find(resource => resource.type === 'database');
  assert.equal(database.image, imageFixture.reference);
  assert.equal(database.source.image, imageFixture.reference);
  assert.equal(database.defaultMountPath, '/var/lib/postgresql/data');
});

test('container runtime entrypoints preserve health ports, non-root users and SIGTERM delivery', async () => {
  const { readFile } = await import('node:fs/promises');
  for (const [resource, port, user] of [[api, '8080', '65532:65532'], [worker, '8081', '65532:65532'], [frontend, '3000', '65532:65532']]) {
    const dockerfile = await readFile(new URL('../' + resource.build.dockerfilePath, import.meta.url), 'utf8');
    const runtime = dockerfile.split(/FROM .+ AS runtime\n/)[1];
    assert.ok(runtime, 'a separate runtime stage is required');
    assert.match(runtime, new RegExp(`^USER ${user}$`, 'm'));
    assert.match(runtime, new RegExp(`^EXPOSE ${port}$`, 'm'));
    assert.equal(resource.variables.PORT.value, port);
    if (resource === frontend) {
      assert.match(runtime, /^ENTRYPOINT \["\/nodejs\/bin\/node"\]$/m);
      assert.match(runtime, /^CMD \["node_modules\/next\/dist\/bin\/next", "start", "--hostname", "0\.0\.0\.0"\]$/m);
      assert.match(runtime, /PORT=3000/);
    } else {
      assert.match(runtime, /^ENTRYPOINT \["\/usr\/local\/bin\/supa-(api|runtime)"\]$/m);
    }
    assert.doesNotMatch(runtime, /^(?:ARG|ENV) (?:OPENAI_API_KEY|SUPA_OPERATOR_CREDENTIALS|SUPA_API_TOKEN)=/m);
    assert.doesNotMatch(dockerfile, /^COPY \.\s/m, 'never import the whole checkout into an image');
  }
});
