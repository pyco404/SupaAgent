import test from 'node:test';
import assert from 'node:assert/strict';
import {assertFresh,runtimeStatus} from './local-launch-demo.mjs';
test('restart preflight rejects expired or nearly expired scopes without renewal',()=>{
 assertFresh({expires_at:131000},100000);
 for(const expires_at of [99999,100000,130000,null,'131000'])assert.throws(()=>assertFresh({expires_at},100000),/no_automatic_renewal/);
});
test('stopped services are reported stopped without contacting any endpoint',async()=>{
 let requests=0;const result=await runtimeStatus({expires_at:0,backend_origin:'http://127.0.0.1:8080'},()=>{requests++;throw Error();});assert.deepEqual(result,{api:'stopped',frontend:'stopped',authorization:'expired'});assert.equal(requests,0);
});
test('status distinguishes health failure from a live URL and exposes no credentials',async()=>{
 const c={expires_at:Date.now()+60000,backend_origin:'http://127.0.0.1:8080',api_pid:1,web_pid:2,database_password:'synthetic'};
 assert.deepEqual(await runtimeStatus(c,async()=>new Response('',{status:503})),{api:'unavailable',frontend:'unavailable',authorization:'valid'});
 assert.deepEqual(await runtimeStatus(c,async()=>new Response('',{status:200})),{api:'ready',frontend:'ready',authorization:'valid'});
});
