#!/usr/bin/env node
/** Local-only synthetic compatibility checks; no provider or database connection. */
import assert from 'node:assert/strict';
import {createHash, randomBytes} from 'node:crypto';
import {spawn, execFileSync} from 'node:child_process';
import {readFile, writeFile, mkdir, realpath} from 'node:fs/promises';
import https from 'node:https';
import {createRequire} from 'node:module';
import net from 'node:net';
import {resolve} from 'node:path';
import {gzipSync, gunzipSync, createGzip} from 'node:zlib';
import {Writable} from 'node:stream';
import {pipeline} from 'node:stream/promises';

const root=resolve(import.meta.dirname,'..');
const [candidateArg, outputArg]=process.argv.slice(2);
assert.ok(candidateArg && outputArg,'candidate extracted-library directory and new output required');
const candidate=await realpath(candidateArg), output=resolve(outputArg);
process.umask(0o077);
await mkdir(output,{mode:0o700});
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
assert.equal(hash(await readFile(process.execPath)), '0adde40d1b7dfa57c8bc83cbc319758c3ec6ebba21363bb2fa752ea6b267ad1b');
const result={node:process.version,candidate_library_sha256:hash(await readFile(resolve(candidate,'libz.so.1.3.1'))),cases:{},database_requests:0,external_requests:0};
assert.equal(result.candidate_library_sha256,'3968a0063bbbbd7d2f2ed6ecd7b3e40c1f3fca398715bc890a5c285f5debceee');
result.next_build_id=(await readFile(resolve(root,'web/.next/BUILD_ID'),'utf8')).trim();
const payload=Buffer.from('synthetic compression input '.repeat(2048));
assert.deepEqual(gunzipSync(gzipSync(payload)),payload);result.cases.node_gzip_roundtrip=true;
await assert.rejects(pipeline([payload],createGzip(),new Writable({write(_c,_e,cb){cb(Object.assign(new Error('synthetic ENOSPC'),{code:'ENOSPC'}));}})),{code:'ENOSPC'});
assert.deepEqual(gunzipSync(gzipSync(payload)),payload);result.cases.stream_failure_and_recovery=true;
const sharp=createRequire(resolve(root,'web/package.json'))('sharp');
const png=await sharp({create:{width:2,height:2,channels:4,background:'#123456'}}).png().toBuffer();
const metadata=await sharp(png).metadata();assert.equal(metadata.width,2);assert.equal(metadata.height,2);result.cases.sharp_png=true;
// Disposable self-signed fixture: never trusted globally or used by any service.
execFileSync('/usr/bin/openssl',['req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=localhost','-addext','subjectAltName=DNS:localhost','-keyout',resolve(output,'synthetic.key'),'-out',resolve(output,'synthetic.crt')],{env:{PATH:'/usr/bin:/bin'},stdio:'ignore'});
const cert=await readFile(resolve(output,'synthetic.crt')),key=await readFile(resolve(output,'synthetic.key'));
const server=https.createServer({cert,key,minVersion:'TLSv1.3'},(_req,res)=>res.end('synthetic TLS'));
await new Promise(r=>server.listen(0,'127.0.0.1',r));
try{
 await new Promise((ok,fail)=>{
  https.get({hostname:'127.0.0.1',port:server.address().port,servername:'localhost',ca:cert,rejectUnauthorized:true},res=>{assert.equal(res.socket.getProtocol(),'TLSv1.3');let data='';res.on('data',c=>data+=c);res.on('end',()=>{try{assert.equal(data,'synthetic TLS');ok();}catch(e){fail(e);}});}).on('error',fail);
 });result.cases.verified_tls13_loopback=true;
}finally{await new Promise(r=>server.close(r));}
const portProbe=net.createServer();await new Promise(r=>portProbe.listen(0,'127.0.0.1',r));const port=portProbe.address().port;await new Promise(r=>portProbe.close(r));
const password=randomBytes(32).toString('hex');
const env={PATH:'/usr/bin:/bin',LC_ALL:'C',NODE_ENV:'production',__NEXT_PROCESSED_ENV:'true',NEXT_TELEMETRY_DISABLED:'1',SUPA_SITE_MODE:'staging',SUPA_SITE_ENV_FILE:'',SUPA_SITE_ORIGIN:`http://127.0.0.1:${port}`,SUPA_BACKEND_URL:'',SUPA_API_TOKEN:'',SUPA_WEB_OPERATORS:'[]',SUPA_STAGING_USERNAME:'synthetic',SUPA_STAGING_PASSWORD:password,LD_LIBRARY_PATH:candidate,SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WORKER_ENABLED:'false',SUPA_WALLET_ENABLED:'false'};
const child=spawn(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(port)],{cwd:resolve(root,'web'),env,stdio:['ignore','ignore','ignore']});
try{
 let ready=false;
 for(let i=0;i<100;i++){try{ready=(await fetch(`http://127.0.0.1:${port}/healthz`)).status===200;}catch{}if(ready)break;if(child.exitCode!==null)break;await new Promise(r=>setTimeout(r,100));}
 assert.ok(ready,'existing production Next.js artifact readiness');
 const base=`http://127.0.0.1:${port}`;
 assert.equal((await fetch(base+'/memory')).status,401);
 const response=await fetch(base+'/?preview=public',{headers:{Authorization:'Basic '+Buffer.from('synthetic:'+password).toString('base64'),'Accept-Encoding':'gzip'}});
 assert.equal(response.status,200);assert.ok((await response.text()).length>1024);
 result.cases.next_production_health_auth_page=true;
 result.next_loaded_zlib=(await readFile(`/proc/${child.pid}/maps`,'utf8')).split('\n').filter(l=>l.includes('libz.so')).map(l=>l.split(/\s+/).at(-1)).filter((v,i,a)=>a.indexOf(v)===i);
}finally{if(child.exitCode===null){child.kill('SIGTERM');await new Promise(r=>child.once('exit',r));}}
result.node_loaded_zlib=process.report.getReport().sharedObjects.filter(v=>v.includes('libz.so'));
assert.deepEqual(result.node_loaded_zlib,[]);assert.deepEqual(result.next_loaded_zlib,[]);
result.library_consumption='Node/Next/Sharp use bundled compression; actual candidate load proven separately by dynamic C regression, not LD_LIBRARY_PATH alone';
result.passed=true;await writeFile(resolve(output,'result.json'),JSON.stringify(result,null,2)+'\n',{mode:0o600});console.log(JSON.stringify(result,null,2));
