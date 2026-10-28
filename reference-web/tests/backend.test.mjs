import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readResource, resource } from '../backend.mjs';
import { records, agent, agentId, financials } from '../../web/tests/fixtures.ts';
import { units } from '../platform.js';

process.env.SUPA_BACKEND_URL='http://127.0.0.1:18082';
process.env.SUPA_API_TOKEN='fixture-token-'.repeat(4);
process.env.SUPA_WEB_USERNAME='fixture';
process.env.SUPA_WEB_PASSWORD='fixture-password-'.repeat(3);
const publicHeaders={host:'127.0.0.1:3202'};
const headers={...publicHeaders,authorization:'Basic '+Buffer.from(`${process.env.SUPA_WEB_USERNAME}:${process.env.SUPA_WEB_PASSWORD}`).toString('base64')};
const response=data=>new Response(JSON.stringify(data),{headers:{'content-type':'application/json'}});

test('public agent projection excludes memory, goals, traits, config and unknown fields',async()=>{
  const result=await readResource('/api/agents',publicHeaders,async()=>response([{...agent,secret_canary:'must-not-forward'}]));
  assert.equal(result.status,200);assert.deepEqual(Object.keys(result.body[0]).sort(),['created_at','id','lifecycle','name']);
  assert.ok(!JSON.stringify(result.body).includes('Browser-test hypothesis'));
});
test('private reads fail closed before making upstream requests',async()=>{
  let calls=0;const fetcher=async()=>{calls++;return response([]);};
  for(const route of ['memory','goals','activity','cycles','actions','financials','simulated-ledger','social','drafts','wallet','wallet/balances','wallet/payments','wallet/treasury']) {
    const result=await readResource(`/api/agents/${agentId}/${route}`,publicHeaders,fetcher);assert.equal(result.status,401);
  }
  assert.equal(calls,0);
});
test('operator reads reuse contracts, remove unknown fields, and only send GET',async()=>{
  const result=await readResource(`/api/agents/${agentId}/financials`,headers,async(url,options)=>{assert.equal(url.pathname,`/agents/${agentId}/financials`);assert.equal(options.method,'GET');assert.equal(options.redirect,'error');assert.ok(options.signal);return response({...financials,private_debug:'hidden'});});
  assert.equal(result.status,200);assert.deepEqual(result.body,financials);
});
test('authorization, query, origin and operation allowlists cannot be widened by commands',async()=>{
  for(const suffix of ['resume','pause','emergency-stop','cycles/run','wallet/sign','wallet/transfer','create','../memory'])assert.equal(resource(`/api/agents/${agentId}/${suffix}`),null);
  assert.equal((await readResource('/api/agents?token=ignored',headers)).status,400);
  assert.equal((await readResource('/api/agents',{...headers,origin:'https://outside.invalid'})).status,403);
  assert.equal((await readResource('/api/agents',{...headers,host:'outside.invalid'})).status,403);
});
test('malformed financial and network evidence is rejected without forwarding response text',async()=>{
  for(const data of [{...financials,balance_microusd:Number.MAX_SAFE_INTEGER+1},{...financials,simulated:false}]){
    const result=await readResource(`/api/agents/${agentId}/financials`,headers,async()=>response(data));assert.deepEqual(result,{status:502,body:{error:'backend_contract_invalid'}});
  }
  const data={network:'mainnet',address:'untrusted'};
  assert.equal((await readResource(`/api/agents/${agentId}/wallet`,headers,async()=>response(data))).status,502);
});
test('upstream credentials/errors, redirects, excessive bodies and outages stay sanitized',async()=>{
  const route='/api/status';
  const fixtures=[async()=>new Response('private upstream body',{status:401}),async()=>new Response('x'.repeat(2_000_001),{headers:{'content-type':'application/json'}}),async()=>{throw Error('private credentials');},async()=>response({status:42})];
  for(const fetcher of fixtures){const result=await readResource(route,headers,fetcher);assert.ok(result.status>=400);assert.ok(!JSON.stringify(result.body).includes('private'));}
});
test('recognized credentials and configured secrets are redacted in approved text fields',async()=>{
  const result=await readResource('/api/agents',headers,async()=>response([{...agent,name:'agent '+process.env.SUPA_API_TOKEN+' Bearer hidden-value'}]));
  assert.ok(result.body[0].name.includes('[redacted]'));assert.ok(!result.body[0].name.includes(process.env.SUPA_API_TOKEN));
});
test('all existing record contracts support authorized reads',async()=>{
  for(const [path,data]of Object.entries(records)){
    if(!path.startsWith('agents/'))continue;
    const result=await readResource('/api/'+path,headers,async()=>response(data));assert.equal(result.status,200,path);
  }
});
test('token amounts use exact integer scaling and never fabricate unavailable balances',()=>{
  assert.equal(units(9007199254740991),'9007199254.740991');assert.equal(units(-1),'−0.000001');assert.equal(units(1,9),'0.000000001');assert.equal(units(null),'unavailable');assert.equal(units(Number.MAX_SAFE_INTEGER+1),'unavailable');
});
