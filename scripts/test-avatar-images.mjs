import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import ts from '../web/node_modules/typescript/lib/typescript.js';
import sharp from '../web/node_modules/sharp/dist/index.mjs';
const src=await readFile(new URL('../web/src/lib/avatar-images.ts',import.meta.url),'utf8');
const image=await import('data:text/javascript,'+encodeURIComponent(ts.transpileModule(src.replace('"sharp"',JSON.stringify(new URL('../web/node_modules/sharp/dist/index.mjs',import.meta.url).href)),{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText));
const out=new URL('../artifacts/avatar-tests/',import.meta.url);await mkdir(out,{recursive:true,mode:0o700});
const seed='11111111-1111-4111-8111-111111111111';
const first=await image.defaultAvatar(seed),second=await image.defaultAvatar('22222222-2222-4222-8222-222222222222');
await writeFile(new URL('first.png',out),first);await writeFile(new URL('second.png',out),second);
const portrait=await sharp({create:{width:160,height:96,channels:3,background:'#71957d'}}).png().toBuffer();await writeFile(new URL('portrait.png',out),portrait);
const noise=Buffer.alloc(512*512*3);let state=1234567;for(let i=0;i<noise.length;i++){state=(Math.imul(state,1664525)+1013904223)>>>0;noise[i]=state>>>24;}await writeFile(new URL('noise.png',out),await image.normalizeAvatar(await sharp(noise,{raw:{width:512,height:512,channels:3}}).png().toBuffer(),'image/png'));
test('default avatars are deterministic, distinct and real metadata-free square PNGs',async()=>{
 assert.deepEqual(first,await image.defaultAvatar(seed));assert.notDeepEqual(first,second);
 const meta=await sharp(first).metadata();assert.equal(meta.width,512);assert.equal(meta.height,512);assert.equal(meta.format,'png');assert.equal(meta.exif,undefined);assert.equal(meta.icc,undefined);
});
test('PNG, JPEG and WebP are fully decoded, normalized, square and stripped',async()=>{
 for(const [format,mime]of [['png','image/png'],['jpeg','image/jpeg'],['webp','image/webp']]){
  const original=await sharp({create:{width:96,height:64,channels:3,background:'#71957d'}}).withMetadata({exif:{IFD0:{Artist:'private-creator-metadata'}}}).toFormat(format).toBuffer();
  const clean=await image.normalizeAvatar(original,mime,{left:16,top:0,size:64});const meta=await sharp(clean).metadata();assert.equal(meta.width,512);assert.equal(meta.height,512);assert.equal(meta.format,'png');assert.equal(meta.exif,undefined);assert.equal(meta.icc,undefined);assert(!clean.includes(Buffer.from('private-creator-metadata')));
 }
});
test('crop position changes real decoded pixels, and oriented JPEG crops remain valid',async()=>{
 const raw=Buffer.alloc(128*64*3);for(let y=0;y<64;y++)for(let x=0;x<128;x++)raw.set(x<64?[255,0,0]:[0,0,255],(y*128+x)*3);
 const bytes=await sharp(raw,{raw:{width:128,height:64,channels:3}}).png().toBuffer();
 const left=await image.normalizeAvatar(bytes,'image/png',{left:0,top:0,size:64}),right=await image.normalizeAvatar(bytes,'image/png',{left:64,top:0,size:64});assert.notDeepEqual(left,right);
 const rgb=await sharp(right).removeAlpha().raw().toBuffer();assert.deepEqual([...rgb.subarray(0,3)],[0,0,255]);
 const oriented=await sharp(bytes).withMetadata({orientation:6}).jpeg().toBuffer();await image.normalizeAvatar(oriented,'image/jpeg',{left:0,top:64,size:64});
});
test('malformed, mislabeled, oversized, animated, vector and invalid crop inputs fail closed',async()=>{
 const rejected=[
  [Buffer.from('<svg><script>alert(1)</script></svg>'),'image/png'],[first,'image/svg+xml'],[first,'image/jpeg'],[Buffer.from('not an image'),'image/webp'],[first.subarray(0,32),'image/png'],[Buffer.alloc(5000001),'image/png'],[first,'image/png',{left:-1,top:0,size:5}],[first,'image/png',{left:500,top:0,size:512}],
 ];
 for(const args of rejected)await assert.rejects(image.normalizeAvatar(...args));
 const wide=await sharp({create:{width:4097,height:1,channels:3,background:'red'}}).png().toBuffer();await assert.rejects(image.normalizeAvatar(wide,'image/png'));
 const chunk=Buffer.alloc(20);chunk.writeUInt32BE(8);chunk.write('acTL',4);const animated=Buffer.concat([first.subarray(0,33),chunk,first.subarray(33)]);await assert.rejects(image.normalizeAvatar(animated,'image/png'));
});
test('streamed byte limits, read deadlines and bounded processing slots remain effective',async()=>{
 await assert.rejects(image.readAvatarBody(new Request('http://127.0.0.1/',{method:'POST',body:Buffer.alloc(5000001)})),e=>e.status===413);
 let release;const pending=new Promise(r=>{release=r;});const a=image.withAvatarSlot(()=>pending),b=image.withAvatarSlot(()=>pending);await assert.rejects(image.withAvatarSlot(async()=>null),e=>e.status===429);release();await Promise.all([a,b]);await image.withAvatarSlot(async()=>null);
 let cancelled=false;const stream=new ReadableStream({pull(){return new Promise(()=>{});},cancel(){cancelled=true;}});await assert.rejects(image.readAvatarBody(new Request('http://127.0.0.1/',{method:'POST',body:stream,duplex:'half'})),e=>e.status===408);assert(cancelled);
});
await writeFile(new URL('manifest.json',out),JSON.stringify({first_sha256:createHash('sha256').update(first).digest('hex'),second_sha256:createHash('sha256').update(second).digest('hex'),fixtures_synthetic:true},null,2)+'\n');
