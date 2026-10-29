import test from 'node:test';
import assert from 'node:assert/strict';
import {sectionView,navigation} from '../platform.js';
import {schemas} from '../../web/src/lib/contracts.ts';
const epoch={id:'42f197d6-7f55-4876-b71f-c23804e0e35c',accounting_namespace:'43f197d6-7f55-4876-b71f-c23804e0e35c',label:'Fresh development epoch',kind:'development',state:'prepared',historical_database:'unrecovered'};
test('health preserves explicit epoch identity and rejects activated or recovered claims',()=>{
 const h={status:'ok',economic_mode:'simulation',epoch,historical_database:'unrecovered'};
 assert.deepEqual(schemas.health.parse(h).epoch,epoch);
 for(const bad of [{...epoch,state:'active'},{...epoch,historical_database:'recovered'},{...epoch,id:'not-a-uuid'}])assert.equal(schemas.health.safeParse({...h,epoch:bad}).success,false);
});
test('all ten prepared views distinguish fresh state without fetching private history or exposing controls',async()=>{
 const before=globalThis.fetch;let calls=0;globalThis.fetch=()=>{calls++;throw Error('unexpected request');};
 try{
  for(const {id} of navigation){
   const rows=await sectionView(id,{status:{data:{epoch}},agents:{data:[]},retrievedAt:'test'},'');
   assert.ok(rows.some(row=>row.text?.includes(epoch.id)&&row.text.includes('historical database unrecovered')));
   assert.equal(rows.some(row=>['autonomy','schedule','live'].includes(row.kind)),false);
  }
  assert.equal(calls,0);
 }finally{globalThis.fetch=before;}
});
test('unidentified and disconnected epochs never imply historical recovery',async()=>{
 for(const status of [{data:{status:'ok'}},{error:'disconnected'}]){
  const rows=await sectionView('agents',{status,agents:{data:[]}},'');
  assert.ok(rows.some(row=>/Epoch (unidentified|unavailable)/.test(row.text??'')));
 }
});
