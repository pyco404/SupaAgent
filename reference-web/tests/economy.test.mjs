import test from 'node:test';
import assert from 'node:assert/strict';
import {runAutonomy,readResource,isEconomicPath} from '../backend.mjs';
import {economicCreateSchema,economicCommandSchema,economicPaymentSchema} from '../../web/src/lib/contracts.ts';
process.env.SUPA_BACKEND_URL='http://127.0.0.1:18082';process.env.SUPA_API_TOKEN='fixture-public-token-'.repeat(3);process.env.SUPA_WEB_USERNAME='fixture';process.env.SUPA_WEB_PASSWORD='fixture-password-'.repeat(3);
const id='1127a114-ad09-4a28-8755-807b52d52008';
const headers={host:'127.0.0.1:3202',origin:'http://127.0.0.1:3202',authorization:'Basic '+Buffer.from(`${process.env.SUPA_WEB_USERNAME}:${process.env.SUPA_WEB_PASSWORD}`).toString('base64'),'content-type':'application/json','x-supa-action':'economic-workflow'};
const input={request_id:id,service_id:id,network:'simulated',input:'Analyze supplied material',valid_for_seconds:3600};
test('economic schemas reject identity, live funds, provider and arbitrary tool overrides',()=>{
 assert.equal(economicCreateSchema.safeParse(input).success,true);
 for(const extra of [{network:'mainnet'},{customer_id:id},{principal_id:id},{tool:'shell'},{input:''},{valid_for_seconds:86401}])assert.equal(economicCreateSchema.safeParse({...input,...extra}).success,false);
 assert.equal(economicCommandSchema.safeParse({request_id:id,expected_revision:0,operation:'transfer',authorize:true}).success,false);
 assert.equal(economicPaymentSchema.safeParse({request_id:id,amount_atoms:1,authorize_simulated:false}).success,false);
 assert.equal(isEconomicPath('/api/jobs/'+id+'/execute-shell'),false);
});
test('economic writes require auth, origin, consent and exact routes before forwarding',async()=>{
 let calls=0;const fetcher=async()=>{calls++;throw Error('offline');};
 for(const changed of [{authorization:''},{origin:'https://outside.invalid'},{'x-supa-action':''}])assert.ok((await runAutonomy('/api/jobs',{...headers,...changed},JSON.stringify(input),fetcher)).status>=400);
 for(const body of [{...input,customer_id:id},{...input,network:'mainnet'}])assert.equal((await runAutonomy('/api/jobs',headers,JSON.stringify(body),fetcher)).status,400);
 assert.equal(calls,0);
 const unknown=await runAutonomy('/api/jobs',headers,JSON.stringify(input),fetcher);assert.equal(calls,1);assert.equal(unknown.body.error,'execution_outcome_unknown_reload');
});
test('economic reads are private and public registry is strictly validated',async()=>{
 let calls=0;const fetcher=async()=>{calls++;return new Response(JSON.stringify({services:[],mainnet_enabled:false}),{headers:{'content-type':'application/json'}});};
 assert.equal((await readResource('/api/agents/'+id+'/economy',{host:headers.host},fetcher)).status,401);assert.equal(calls,0);
 assert.equal((await readResource('/api/services',{host:headers.host},fetcher)).status,200);assert.equal(calls,1);
});
