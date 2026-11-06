#!/usr/bin/env node
/** Actual Next.js boundary rehearsal. Synthetic credentials; no database or provider. */
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {createServer, request} from 'node:http';
import {resolve} from 'node:path';
const web=process.env.SUPA_TEST_WEB_ROOT??resolve(import.meta.dirname,'../web');
const password=randomBytes(32).toString('hex'), operatorPassword=randomBytes(32).toString('hex');
const basic=(name,secret)=>'Basic '+Buffer.from(`${name}:${secret}`).toString('base64');
let checks=0;
async function exercise(enabled){
 const probe=createServer();await new Promise(r=>probe.listen(0,'127.0.0.1',r));const port=probe.address().port;await new Promise(r=>probe.close(r));
 const child=spawn(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(port)],{cwd:web,stdio:'ignore',env:{PATH:'/usr/local/bin:/usr/bin:/bin',NODE_ENV:'production',NEXT_TELEMETRY_DISABLED:'1',__NEXT_PROCESSED_ENV:'true',SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:`http://127.0.0.1:${port}`,SUPA_PUBLIC_LANDING_ENABLED:String(enabled),SUPA_BACKEND_URL:'',SUPA_API_TOKEN:'',SUPA_SITE_ENV_FILE:'',SUPA_STAGING_USERNAME:'fixture-visitor',SUPA_STAGING_PASSWORD:password,SUPA_WEB_OPERATORS:JSON.stringify([{username:'fixture-operator',password:operatorPassword,api_token:randomBytes(32).toString('hex')}])}});
 const get=(path,host='supaagency.xyz',headers={},method='GET')=>new Promise((yes,no)=>{
  const req=request({hostname:'127.0.0.1',port,path,method,headers:{host,...headers},timeout:5000},res=>{let text='';res.setEncoding('utf8');res.on('data',chunk=>{text+=chunk;if(text.length>1024*1024)req.destroy(new Error('response_bound'));});res.on('end',()=>yes({status:res.statusCode,headers:res.headers,text}));});req.on('timeout',()=>req.destroy(new Error('request_timeout')));req.on('error',no);req.end();
 });
 const check=async(path,status,host,headers,method)=>{const r=await get(path,host,headers,method);assert.equal(r.status,status,`boundary_${enabled}_${path}_${method??'GET'}`);checks++;return r;};
 try{
  let ready=false;for(let i=0;i<100;i++){try{ready=(await get('/healthz',`127.0.0.1:${port}`)).status===200;}catch{}if(ready)break;if(child.exitCode!==null)break;await new Promise(r=>setTimeout(r,100));}assert.ok(ready,'next_readiness');
  await check('/',401,`127.0.0.1:${port}`);
  await check('/',401,`127.0.0.1:${port}`,{'x-supa-public-landing':'1'});
  if(!enabled){await check('/',403);await check('/',403,'www.supaagency.xyz');return;}
  const landing=await check('/',200);
  for(const text of ['Supa Agency','One agent. One purpose. One token.','https://supaagency.xyz/','data-public-landing="true"','href="/operator"'])assert.ok(landing.text.includes(text),'landing_brand_contract');
  assert.equal(landing.headers['x-robots-tag'],'index, follow');assert.match(landing.text, /name="robots" content="index, follow"/);
  await check('/',200,undefined,{},'HEAD');
  for(const path of ['/api/status','/api/agents','/api/operator-status','/api/autonomy','/operator','/agents','/memory','/economy','/experience','/experience/api/status','/.env','/public/secrets.env'])await check(path,401);
  await check('/agents',401,undefined,{'x-supa-public-landing':'1','x-supa-experience':'1'});
  await check('/api/autonomy',401,undefined,{},'POST');
  await check('/',401,undefined,{},'POST');
  await check('/',403,undefined,{origin:'https://untrusted.invalid'});
  await check('/',403,'supaagency.xyz.evil.invalid');
  for(const path of ['/app.js','/scene.js','/wheel.js','/platform.js','/style.css','/public/assets/supa-agent-icon.svg','/public/assets/supa-agent-code.svg','/public/fonts/IBMPlexMono-Regular.ttf','/robots.txt'])await check(path,200);
  const redirect=await check('/?test=1',308,'www.supaagency.xyz');assert.equal(redirect.headers.location,'https://supaagency.xyz/?test=1');
  const visitor={authorization:basic('fixture-visitor',password)};
  const signed=await check('/',200,`127.0.0.1:${port}`,visitor);assert.ok(!signed.text.includes('data-public-landing="true"'),'staging_not_public');
  await check('/operator',401,undefined,visitor);
  const operator=await check('/operator',302,undefined,{authorization:basic('fixture-operator',operatorPassword)});assert.equal(operator.headers.location,'/agents');
 }finally{
  if(child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await new Promise((yes,no)=>{const t=setTimeout(()=>{child.kill('SIGKILL');no(new Error('shutdown_timeout'));},5000);child.once('exit',()=>{clearTimeout(t);yes();});});}
 }
}
try{await exercise(false);await exercise(true);console.log(JSON.stringify({passed:true,checks,public_entry_opt_in:true,staging_authentication_preserved:true,private_routes_protected:true,database_connections:0,paid_requests:0}));}
catch(error){console.error(JSON.stringify({passed:false,stage:'public_landing_boundary',type:error instanceof assert.AssertionError?'AssertionError':'RuntimeError',assertion:error instanceof assert.AssertionError&&/^\w[\w/?. _-]{0,120}$/.test(error.message.split('\n')[0])?error.message.split('\n')[0]:'boundary_rehearsal_failed'}));process.exitCode=1;}
