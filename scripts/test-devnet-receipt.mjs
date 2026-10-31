import test from 'node:test';
import assert from 'node:assert/strict';
import {address,target,MINT,run} from './devnet-receipt.mjs';
import {resolve} from 'node:path';
const root=resolve(import.meta.dirname,'..');
test('recipient and exact canonical mint contain only public identities',()=>{assert.equal(address('11111111111111111111111111111112'),'11111111111111111111111111111112');for(const bad of ['','0'.repeat(32),'1'.repeat(31),'1'.repeat(33),'private-key'])assert.throws(()=>address(bad));assert.equal(MINT,'4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU');});
test('runner rejects primary remote arbitrary schema and credential targets',()=>{
 const query=new URLSearchParams({host:root+'/postgres-data/devnet-receipt-readiness/unit/socket',port:'55450'});const valid='postgresql://pikoo@localhost/supa_devnet_receipt_test?'+query;assert.equal(target(valid).socket,root+'/postgres-data/devnet-receipt-readiness/unit/socket');
 for(const bad of [valid.replace('localhost','remote.invalid'),valid.replace('localhost','localhost:5432'),valid.replace('pikoo@','other@'),valid.replace('supa_devnet_receipt_test','supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748'),valid.replace('55450','55440'),valid+'&options=unsafe',valid.replace('pikoo@','pikoo:forbidden@'),valid.replace('devnet-receipt-readiness','infrastructure-v1')])assert.throws(()=>target(bad));
});
test('unassigned preparation performs no database or network operation',async()=>{const before=process.env.TEST_DATABASE_URL;delete process.env.TEST_DATABASE_URL;try{await run(['prepare']);}finally{if(before!==undefined)process.env.TEST_DATABASE_URL=before;}});
