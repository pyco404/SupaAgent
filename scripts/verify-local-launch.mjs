/** Read genuine disposable outcomes and test scoped role boundaries; no provider requests. */
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';
const out=new URL('../artifacts/local-launch/',import.meta.url),c=JSON.parse(await readFile(new URL('runtime.json',out),'utf8'));
assert.equal(c.purpose,'disposable_local_agent_launch');assert.match(c.database,/^supa_launch_[a-f0-9]{12}_test$/);assert.equal(c.backend_origin,'http://127.0.0.1:8080');
async function call(path,account=c.accounts[0],body,extra={}){const r=await fetch(c.backend_origin+path,{method:body===undefined?'GET':'POST',headers:{Authorization:'Bearer '+account.token,'Content-Type':'application/json',...extra},...(body===undefined?{}:{body:JSON.stringify(body)}),signal:AbortSignal.timeout(5000)});return {status:r.status,body:await r.json()};}
const status=await call('/operator-status');assert.equal(status.body.database,c.database);assert.equal(status.body.epoch,null);
const directory=await call('/agents');assert.equal(directory.status,200);const agent=directory.body.find(a=>a.name==='Supa Observatory');assert.ok(agent);const id=agent.id;
const records={};for(const key of ['schedule','cycles','memory','goals','actions','activity','financials','simulated-ledger','authorizations','live']){const r=await call(`/agents/${id}/${key}`);assert.equal(r.status,200,key);records[key]=r.body;}
assert.equal(agent.lifecycle,'terminated');assert.equal(records.schedule.enabled,false);assert.equal(records.live.enabled,false);
assert.ok(records.cycles.length>=3&&records.cycles.length<=6);assert.ok(records.cycles.every(c=>c.status==='completed'));
assert.equal(records.schedule.cycles_started,records.cycles.length);assert.equal(records.schedule.policy.max_cycles,6);assert.equal(records.schedule.policy.simulated_budget_microusd,6600);
assert.ok(records.memory.length>0);assert.ok(records.goals.every(g=>!g.completed));
assert.equal(records.financials.revenue_microusd,0);assert.equal(records.financials.balance_microusd,20000-records.cycles.length*1100);
assert.equal(records.financials.inference_microusd+records.financials.operating_microusd,records.cycles.length*1100);
assert.equal(records.authorizations.length,1,'resume must not create a new allowance');
assert.equal((await call(`/agents/${id}/memory`,c.accounts[3])).status,403);
// Explicitly grant synthetic operator/viewer read scopes on this test identity only.
for(const account of c.accounts.slice(1,3)){const prior=await call(`/agents/${id}/memory`,account);assert.ok([200,403].includes(prior.status));if(prior.status===403)assert.equal((await call(`/agents/${id}/access`,c.accounts[0],{principal_id:account.id,role:account.role})).status,200);assert.equal((await call(`/agents/${id}/access`,account)).body.role,account.role);assert.equal((await call(`/agents/${id}/memory`,account)).status,200);}
const command={request_id:randomUUID(),expected_revision:records.schedule.revision,operation:'start',authorize_offline:true,reason:'Permission-negative test; must not start.',policy:{goal_ids:records.goals.map(g=>g.id),interval_seconds:10,max_cycles:1,simulated_budget_microusd:1100,valid_for_seconds:60}};
assert.equal((await call(`/agents/${id}/schedule`,c.accounts[1],command)).status,403,'operator cannot authorize new schedule');
assert.equal((await call(`/agents/${id}/schedule`,c.accounts[2],command,{'X-Supa-Principal':c.accounts[0].id,'X-Supa-Role':'owner'})).status,403,'viewer cannot spoof owner');
assert.equal((await call(`/agents/${id}/memory`,c.accounts[3])).status,403,'unrelated owner cannot read private memory');
const resume=await call(`/agents/${id}/schedule`,c.accounts[0],{...command,request_id:randomUUID(),operation:'resume',policy:null});assert.ok([400,409].includes(resume.status),'terminated identity cannot resume');
const before=records.cycles.length;await new Promise(r=>setTimeout(r,1500));assert.equal((await call(`/agents/${id}/cycles`)).body.length,before);assert.equal((await call(`/agents/${id}/financials`)).body.balance_microusd,records.financials.balance_microusd);
const result={passed:true,agent_id:id,offline_cycles:before,memory_records:records.memory.length,tool_actions:records.actions.length,activity_events:records.activity.length,goals_open:records.goals.length,lifecycle:'terminated',simulated_seed_microusd:20000,simulated_expenses_microusd:before*1100,simulated_revenue:0,paid_requests:0,finite_authorizations:1,role_boundaries_verified:true,spoofing_rejected:true,terminal_stop_fenced:true};
await writeFile(new URL('system-validation.json',out),JSON.stringify(result,null,2)+'\n',{mode:0o600});console.log(JSON.stringify(result));
