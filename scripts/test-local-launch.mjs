import assert from 'node:assert/strict';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {randomUUID} from 'node:crypto';
import {launch} from '../reference-web/launch.mjs';
const dir=await mkdtemp(join(tmpdir(),'supa-launch-unit-'));
const original={...process.env};
try{
const token='synthetic-launch-owner-token-000000000000000',password='synthetic-launch-owner-password-00000000000';
process.env.SUPA_WEB_OPERATORS=JSON.stringify([{username:'fixture',password,api_token:token}]);
process.env.SUPA_BACKEND_URL='http://127.0.0.1:55711';process.env.SUPA_SITE_MODE='local';
process.env.SUPA_LOCAL_LAUNCH_SCOPE=join(dir,'scope.json');
await writeFile(process.env.SUPA_LOCAL_LAUNCH_SCOPE,JSON.stringify({purpose:'disposable_local_agent_launch',primary_mutations_authorized:false,expires_at:Date.now()+60000,backend_origin:process.env.SUPA_BACKEND_URL,database:'supa_launch_0123456789ab_test'}),{mode:0o600});
const headers={host:'127.0.0.1:3200',origin:'http://127.0.0.1:3200',authorization:'Basic '+Buffer.from('fixture:'+password).toString('base64'),'content-type':'application/json','x-supa-action':'offline-launch'};
const input=()=>({request_id:randomUUID(),name:'Test agent',personality:'Curious and careful.',goal:'Observe verified execution records.',simulated_budget_microusd:10000,authorize_offline:true});
let calls=[],role='owner',epoch=null,goalFailure=false;
async function fetcher(url,options){
 const path=url.pathname;calls.push({path,options});
 if(path==='/operator-status')return Response.json({database:'supa_launch_0123456789ab_test',epoch,role});
 if(path==='/agents'){const value=JSON.parse(options.body);assert.deepEqual(value.capabilities,['read_memory','remember','evaluate_goal']);assert.equal(value.config.scheduler_enabled,false);return Response.json({id:'11111111-1111-4111-8111-111111111111',lifecycle:'created'});}
 if(path.endsWith('/goals'))return goalFailure?Response.json({}, {status:403}):Response.json({id:randomUUID()});
 if(path.endsWith('/simulated-funding')){const value=JSON.parse(options.body);assert.equal(value.kind,'seed');assert.equal(value.simulated,true);return Response.json({balance_microusd:value.amount_microusd});}
 throw Error('unapproved_route');
}
assert.equal((await launch('/api/launch',{},undefined,fetcher)).status,403);
assert.equal((await launch('/api/launch',{host:headers.host},undefined,fetcher)).status,401);
assert.equal((await launch('/api/launch',{...headers,origin:'http://outside.invalid'},JSON.stringify(input()),fetcher)).status,403);
role='viewer';assert.equal((await launch('/api/launch',headers,undefined,fetcher)).body.enabled,false);assert.equal((await launch('/api/launch',headers,JSON.stringify(input()),fetcher)).status,403);
role='owner';epoch={state:'prepared'};assert.equal((await launch('/api/launch',headers,JSON.stringify(input()),fetcher)).status,503);epoch=null;
for(const change of [{authorize_offline:false},{simulated_budget_microusd:99999999},{owner_id:randomUUID()},{personality:'ab'}])assert.equal((await launch('/api/launch',headers,JSON.stringify({...input(),...change}),fetcher)).status,400);
calls=[];const value=input();assert.equal((await launch('/api/launch',headers,JSON.stringify(value),fetcher)).status,200);assert.equal(calls.length,4);assert.equal((await launch('/api/launch',headers,JSON.stringify(value),fetcher)).status,409);assert.equal(calls.filter(c=>c.path==='/agents').length,1);
goalFailure=true;const partial=await launch('/api/launch',headers,JSON.stringify(input()),fetcher);assert.equal(partial.status,403);assert.equal(partial.body.stage,'adding_goal');assert.ok(partial.body.agent_id);
assert.ok(calls.every(c=>!c.path.includes('live')&&!c.path.includes('cycles')&&!c.path.includes('schedule')));
console.log('PASS: local/origin/auth/role/epoch/schema/budget boundaries; inactive creation; replay fencing; partial-state recovery; no execution/provider routes');
}finally{process.env=original;await rm(dir,{recursive:true,force:true});}
