import {test} from 'node:test';
import assert from 'node:assert/strict';
import {publicNavigation,publicSectionView,read} from '../platform.js';

test('public projection is a fixed editorial allowlist with no live record input',async()=>{
 const saved=globalThis.fetch;let requests=0;globalThis.fetch=()=>{requests++;throw Error('unexpected_network');};
 try{
  assert.deepEqual(publicNavigation.map(x=>x.path),['/home', '/agents', '/intelligence', '/memory', '/economy', '/social', '/trading', '/factory', '/about']);
  for(const item of publicNavigation){
   const rows=publicSectionView(item.id,{agents:[{name:'PRIVATE_STAGING_SENTINEL'}]});
   assert.deepEqual(rows,publicSectionView(item.id));
   assert.ok(rows.every(row=>['command','notice','text','heading','navigate','link','disabled','github','artwork'].includes(row.kind)));
   assert.ok(!JSON.stringify(rows).includes('PRIVATE_STAGING_SENTINEL'));
  }
  assert.match(JSON.stringify(publicSectionView('agents')),/No public agent profiles are available yet/);
  assert.match(JSON.stringify(publicSectionView('home')),/Supa Agent/);
  assert.match(JSON.stringify(publicSectionView('about')),/planned, not live/);
  assert.equal(publicSectionView('memory').some(x=>x.kind==='detail'),false);
  for(const id of ['intelligence','memory','economy','social','trading','token-launch','factory']){
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

test('Factory separates agent creation, qualification and optional permanent tokens',()=>{
 for(const id of ['home','agents'])assert.ok(publicSectionView(id).some(row=>row.kind==='navigate'&&row.id==='factory'&&row.text==='Create Agent →'));
 const rows=publicSectionView('factory'),text=rows.map(row=>row.text).join('\n');
 assert.deepEqual(rows.filter(row=>row.kind==='heading').map(row=>row.text),['Factory / Create Agents','1. Create Agent','2. Configure','3. Purpose Qualification','4. Agent Token / Optional','5. My Agents']);
 for(const expected of ['Name, purpose, description and personality','autonomy level and spending limits','creator consent','Skip this step','Pump.fun','platform token $SUPA','permanently','public disclosure','no automatic wallet control'])assert.ok(text.includes(expected),expected);
 assert.equal(rows.filter(row=>row.kind==='disabled').length,4);
 assert.ok(rows.some(row=>row.href==='/my-agents'));
 assert.ok(!rows.some(row=>['launch','agent','schedule','live','economy'].includes(row.kind)));
 assert.equal(publicNavigation.some(row=>row.id==='token-launch'||row.id==='my-agents'),false);
});

test('synthetic Factory offers sign-in without collecting public input or enabling execution',()=>{
 const doc=globalThis.document,fetch=globalThis.fetch;
 globalThis.document={documentElement:{dataset:{publicLanding:'true',syntheticFactory:'true'}}};
 globalThis.fetch=()=>{throw Error('public_factory_must_not_read_backend');};
 try{
  const rows=publicSectionView('factory'),text=rows.map(r=>r.text).join('\n');
  assert.ok(rows.some(r=>r.kind==='link'&&r.href==='/sign-in'&&r.text==='Create Agent / synthetic sign-in'));
  assert.match(text,/save a private paused agent/);assert.match(text,/only after sign-in and private-storage consent/);
  assert.equal(rows.filter(r=>r.kind==='disabled').length,4);
  assert.ok(!rows.some(r=>['agent','launch','schedule','live','form'].includes(r.kind)));
 }finally{globalThis.fetch=fetch;if(doc===undefined)delete globalThis.document;else globalThis.document=doc;}
});

// Editorial placement is explicit public content; catalog entries alone never create images.
test('public sections deliberately select zero, one or multiple approved illustrations',()=>{
 const expected={home:['home'],agents:['agents'],intelligence:[],memory:[],economy:[],social:[],trading:[],factory:['factory','intelligence'],about:[]};
 for(const [id,art]of Object.entries(expected)){
  const rows=publicSectionView(id);assert.deepEqual(rows.filter(row=>row.kind==='artwork').map(row=>row.id),art);
  for(const row of rows.filter(row=>row.kind==='artwork')){const index=rows.indexOf(row);assert.ok(index>2);assert.equal(rows[index-1].kind,'text');assert.equal(Object.keys(row).length,2);}
 }
 assert.deepEqual(publicSectionView('unknown').filter(row=>row.kind==='artwork'),[]);
});
