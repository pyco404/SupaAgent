import {test} from 'node:test';
import assert from 'node:assert/strict';
import {runAutonomy,readResource} from '../backend.mjs';
process.env.SUPA_BACKEND_URL='http://127.0.0.1:18082';process.env.SUPA_API_TOKEN='fixture-token-'.repeat(4);
process.env.SUPA_WEB_USERNAME='fixture';process.env.SUPA_WEB_PASSWORD='fixture-password-'.repeat(3);
const id='11111111-1111-4111-8111-111111111111';
const body={request_id:id,goal:'Learn from verified outcomes',authorize_offline:true};
const headers={host:'127.0.0.1:3202',origin:'http://127.0.0.1:3202','content-type':'application/json','x-supa-action':'offline-autonomy',authorization:'Basic '+Buffer.from(`${process.env.SUPA_WEB_USERNAME}:${process.env.SUPA_WEB_PASSWORD}`).toString('base64')};
const result={agent_id:id,mode:'offline',simulated:true,cycles:[id,'22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333'],lifecycle:'paused',paid_requests:0,simulated_seed_microusd:10000,simulated_expense_limit_microusd:3300};
const response=data=>Response.json(data);
test('workflow authorization and same-origin checks precede every upstream call',async()=>{
  let calls=0;const fetcher=async()=>{calls++;return response(result);};
  for(const altered of [{authorization:''},{origin:''},{origin:'https://outside.invalid'},{host:'outside.invalid'},{'x-supa-action':''},{'content-type':'text/plain'}])assert.ok((await runAutonomy('/api/autonomy',{...headers,...altered},JSON.stringify(body),fetcher)).status>=400);
  assert.equal((await readResource('/api/autonomy',{host:headers.host},fetcher)).status,401);
  assert.equal((await runAutonomy('/api/agents',headers,JSON.stringify(body),fetcher)).status,403);assert.equal(calls,0);
});
test('strict body prevents paid provider, permission, budget and goal overrides',async()=>{
  let calls=0;const fetcher=async()=>{calls++;return response(result);};
  for(const payload of [{...body,authorize_offline:false},{...body,provider:'openai'},{...body,simulated_seed_microusd:999999},{...body,capabilities:['transfer']},{...body,goal:''},{...body,request_id:'bad'}])assert.equal((await runAutonomy('/api/autonomy',headers,JSON.stringify(payload),fetcher)).status,400);
  assert.equal((await runAutonomy('/api/autonomy',headers,'x'.repeat(4097),fetcher)).status,413);assert.equal(calls,0);
});
test('successful workflow forwards only fixed JSON and declassifies validated outcome fields',async()=>{
  const output=await runAutonomy('/api/autonomy',headers,JSON.stringify(body),async(url,options)=>{
    assert.equal(url.pathname,'/autonomy');assert.equal(options.method,'POST');assert.equal(options.redirect,'error');assert.deepEqual(JSON.parse(options.body),body);return response({...result,private_context:'must not leave server'});
  });assert.equal(output.status,200);assert.deepEqual(output.body,result);
  for(const changed of [{mode:'openai'},{paid_requests:1},{lifecycle:'active'},{cycles:[id,id,id]},{agent_id:'22222222-2222-4222-8222-222222222222'}])assert.equal((await runAutonomy('/api/autonomy',headers,JSON.stringify(body),async()=>response({...result,...changed}))).status,502);
});
test('unknown outcomes and conflicts preserve request identity and never retry',async()=>{
  let calls=0;const out=await runAutonomy('/api/autonomy',headers,JSON.stringify(body),async()=>{calls++;throw Error('sensitive transport details');});
  assert.equal(calls,1);assert.equal(out.body.agent_id,id);assert.equal(out.status,503);assert.ok(!JSON.stringify(out).includes('sensitive'));
  const conflict=await runAutonomy('/api/autonomy',headers,JSON.stringify(body),async()=>new Response('private',{status:409}));assert.equal(conflict.status,409);assert.equal(conflict.body.agent_id,id);assert.ok(!JSON.stringify(conflict).includes('private'));
});
