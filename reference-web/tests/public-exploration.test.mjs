import {test} from 'node:test';
import assert from 'node:assert/strict';
import {publicNavigation,publicSectionView,read} from '../platform.js';

test('public projection is a fixed editorial allowlist with no live record input',async()=>{
 const saved=globalThis.fetch;let requests=0;globalThis.fetch=()=>{requests++;throw Error('unexpected_network');};
 try{
  assert.deepEqual(publicNavigation.map(x=>x.path),['/home', '/agents', '/intelligence', '/memory', '/economy', '/social', '/trading', '/token-launch', '/factory', '/about']);
  for(const item of publicNavigation){
   const rows=publicSectionView(item.id,{agents:[{name:'PRIVATE_STAGING_SENTINEL'}]});
   assert.deepEqual(rows,publicSectionView(item.id));
   assert.ok(rows.every(row=>['command','notice','text','heading','navigate','link','disabled'].includes(row.kind)));
   assert.ok(!JSON.stringify(rows).includes('PRIVATE_STAGING_SENTINEL'));
  }
  assert.match(JSON.stringify(publicSectionView('agents')),/No public agent profiles are available yet/);
  assert.match(JSON.stringify(publicSectionView('home')),/Supa Agent/);
  assert.match(JSON.stringify(publicSectionView('about')),/planned, not live/);
  assert.equal(publicSectionView('memory').some(x=>x.kind==='detail'),false);
  for(const id of ['agents','intelligence','memory','economy','social','trading','token-launch','factory']){
   assert.ok(publicSectionView(id).some(row=>row.kind==='disabled'),`${id}_no_executable_controls`);
   assert.ok(!publicSectionView(id).some(row=>['agent','detail','draft','launch','economy','schedule','live'].includes(row.kind)));
  }
  assert.equal(requests,0);
 }finally{globalThis.fetch=saved;}
});
test('public browser mode refuses private reads before any network request',async()=>{
 const doc=globalThis.document,fetch=globalThis.fetch;let requests=0;
 globalThis.document={documentElement:{dataset:{publicLanding:'true'}}};
 globalThis.fetch=()=>{requests++;throw Error('unexpected_network');};
 try{for(const path of ['/api/agents','/api/operator-status','/api/agents/private/memory'])assert.deepEqual(await read(path),{error:'Private records require sign-in.'});assert.equal(requests,0);}
 finally{if(doc===undefined)delete globalThis.document;else globalThis.document=doc;globalThis.fetch=fetch;}
});
