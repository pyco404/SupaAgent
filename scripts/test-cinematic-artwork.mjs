import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import sharp from '../web/node_modules/sharp/dist/index.mjs';
import {sectionArtwork,navigation} from '../reference-web/platform.js';
import {publicLandingRequest} from '../web/src/lib/auth.ts';

test('nine original scenes have complete optimized square variants with recorded integrity',async()=>{
 const inventory=JSON.parse(await readFile(new URL('../docs/cinematic-artwork-inventory.json',import.meta.url),'utf8'));
 assert.deepEqual(inventory.assets.map(x=>x.section),Object.keys(sectionArtwork));
 assert.equal(inventory.assets.length,9);let total=0;
 for(const asset of inventory.assets){
  assert.match(asset.sourceSha256,/^[0-9a-f]{64}$/);assert.equal(asset.files.length,4);
  for(const file of asset.files){
   assert.match(file.path,/^reference-web\/public\/assets\/supa-[a-z]+-(thumb|480|960)\.(webp|avif)$/);
   const bytes=await readFile(new URL('../'+file.path,import.meta.url));total+=bytes.length;
   assert.equal(createHash('sha256').update(bytes).digest('hex'),file.sha256);assert.equal(bytes.length,file.bytes);
   const metadata=await sharp(bytes).metadata();
   assert.equal(metadata.width,file.width);assert.equal(metadata.height,file.height);
   assert.equal(metadata.width,metadata.height);assert.ok(!metadata.exif&&!metadata.xmp&&!metadata.icc);
   assert.ok(bytes.length<160_000);
  }
 }
 assert.ok(total<1_250_000);
 assert.deepEqual(navigation.map(x=>x.thumb),Object.keys(sectionArtwork).map(id=>`/public/assets/supa-${id}-thumb.webp`));
});

test('public artwork allowlist does not grant access to uploads private routes or other hosts',()=>{
 Object.assign(process.env,{SUPA_SITE_MODE:'staging',SUPA_PUBLIC_LANDING_ENABLED:'true'});
 const request=(path,host='supaagency.xyz',method='GET',origin)=>new Request('https://'+host+path,{method,headers:{host,...(origin?{origin}:{})}});
 for(const id of Object.keys(sectionArtwork))for(const variant of ['thumb.webp','480.webp','960.webp','960.avif']){
  const path=`/public/assets/supa-${id}-${variant}`;
  assert.equal(publicLandingRequest(request(path)),true);
  assert.equal(publicLandingRequest(request(path,'supaagency.xyz','HEAD')),true);
  assert.equal(publicLandingRequest(request(path,'supa-web-staging.up.railway.app')),false);
  assert.equal(publicLandingRequest(request(path,'untrusted.invalid')),false);
  assert.equal(publicLandingRequest(request(path,'supaagency.xyz','POST')),false);
  assert.equal(publicLandingRequest(request(path,'supaagency.xyz','GET','https://untrusted.invalid')),false);
 }
 for(const path of ['/operator','/my-agents','/api/operator-status','/api/factory/avatar-uploads','/public/assets/private.webp','/public/assets/supa-home-thumb.png','/public/assets/supa-home-thumb.webp/private','/public/assets/supa-home-thumb.webp%00','/docs/cinematic-artwork-inventory.json'])assert.equal(publicLandingRequest(request(path)),false,path);
 process.env.SUPA_PUBLIC_LANDING_ENABLED='false';assert.equal(publicLandingRequest(request('/public/assets/supa-home-480.webp')),false);
});

test('existing logo linework and disabled generation remain separate from editorial imagery',async()=>{
 const html=await readFile(new URL('../reference-web/index.html',import.meta.url),'utf8');
 for(const value of ['class="wheel"','class="wheel-ink"','class="blueprint"','class="workspace-hands"','class="buffer"','supa-agent-code.svg'])assert.ok(html.includes(value));
 const factory=await readFile(new URL('../reference-web/factory-create.html',import.meta.url),'utf8');
 assert.match(factory,/Generate with AI \/ unavailable/);assert.match(factory,/disabled/);
 const profile=await readFile(new URL('../reference-web/public-agent-profile.html',import.meta.url),'utf8');
 assert.ok(!profile.includes('supa-agents-'),'editorial portrait must not impersonate an approved persisted agent avatar');
});
