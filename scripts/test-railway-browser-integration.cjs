#!/usr/bin/env node
'use strict';
// Run the live acceptance entrypoint against disposable Next.js, with only
// credential loading, the official Host adapter and transport mapped locally.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const http=require('node:http');
const {spawn}=require('node:child_process');
const {randomBytes}=require('node:crypto');
const APP=path.resolve(__dirname,'..');
const web=fs.realpathSync(process.env.SUPA_TEST_WEB_ROOT??'');
assert.ok(web.startsWith('/tmp/'),'disposable_web_root_required');
const out=path.resolve(process.env.SUPA_TEST_EVIDENCE??path.join(APP,'artifacts/browser-probe-integration'));
fs.mkdirSync(out,{recursive:true,mode:0o700});
const fixtureValue=()=>randomBytes(32).toString('hex');
const password=fixtureValue();
const accounts=['viewer','operator'].map(role=>({username:role,password,api_token:fixtureValue()}));
const payload={data:{web:{SUPA_WEB_OPERATORS:JSON.stringify(accounts)},api:{
  SUPA_OPERATOR_CREDENTIALS:JSON.stringify(accounts.map(a=>({id:a.username,role:a.username,token:a.api_token}))),
  SUPA_OPERATOR_SESSION_DEADLINES:JSON.stringify({viewer:Date.now()/1000+3600,operator:Date.now()/1000+3600})}}};
let next,child,adapter,fault;
const freePort=async()=>{const server=http.createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;await new Promise(r=>server.close(r));return port;};
async function close(server){if(server){server.closeAllConnections();await new Promise(r=>server.close(r));}}
(async()=>{
 try{
  const port=await freePort();
  next=spawn(process.execPath,['node_modules/next/dist/bin/next','dev','--webpack','--hostname','127.0.0.1','--port',String(port)],{
    cwd:web,detached:true,stdio:'ignore',env:{PATH:'/usr/bin:/bin',NODE_ENV:'development',NEXT_TELEMETRY_DISABLED:'1',__NEXT_PROCESSED_ENV:'true',
      SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:'https://supa-web-staging.up.railway.app',SUPA_PUBLIC_LANDING_ENABLED:'true',
      SUPA_BACKEND_URL:'',SUPA_API_TOKEN:'',SUPA_SITE_ENV_FILE:'',SUPA_WEB_OPERATORS:JSON.stringify(accounts),
      SUPA_STAGING_USERNAME:'synthetic-visitor',SUPA_STAGING_PASSWORD:fixtureValue()}});
  let ready=false;for(let n=0;n<120;n++){
    try{const r=await fetch('http://127.0.0.1:'+port+'/healthz');ready=r.status===200;await r.body?.cancel();}catch{}
    if(ready||next.exitCode!==null)break;await new Promise(r=>setTimeout(r,250));
  }
  assert.ok(ready,'disposable_next_readiness');
  let browserOrigin;
  adapter=http.createServer((req,res)=>{
    const targetHost=req.headers['x-test-acceptance-host']??'supaagency.xyz';
    assert.ok(['supaagency.xyz','supa-web-staging.up.railway.app'].includes(targetHost));
    const headers={...req.headers,host:targetHost};delete headers['x-test-acceptance-host'];
    if(headers.origin===browserOrigin)headers.origin='https://supaagency.xyz';
    const upstream=http.request({hostname:'127.0.0.1',port,path:req.url,method:req.method,headers},r=>{res.writeHead(r.statusCode,r.headers);r.pipe(res);});
    upstream.on('error',()=>res.writeHead(502).end());req.pipe(upstream);
  });
  await new Promise(r=>adapter.listen(0,'127.0.0.1',r));browserOrigin='http://127.0.0.1:'+adapter.address().port;
  fault=http.createServer(req=>req.socket.destroy());await new Promise(r=>fault.listen(0,'127.0.0.1',r));
  const prelude=`
const fs=require('node:fs'),Module=require('node:module'),assert=require('node:assert/strict');
const real=require(${JSON.stringify(path.join(APP,'scripts/railway-browser-observation.cjs'))});
const filename=${JSON.stringify(path.join(APP,'scripts/railway-browser-acceptance.cjs'))};
let source=fs.readFileSync(filename,'utf8');
assert.ok(source.includes("const origin='https://supaagency.xyz';"));
source=source.replace("const origin='https://supaagency.xyz';",'const origin='+${JSON.stringify(JSON.stringify(browserOrigin))}+';');
source=source.replace("stage='protected_routes';subtest='protected_routes';","stage='protected_routes';subtest='protected_routes';globalThis.finalProbeStage=true;");
let injected=false;
const moduleObject=new Module(filename,module);moduleObject.filename=filename;moduleObject.paths=module.paths;
const native=moduleObject.require.bind(moduleObject);
moduleObject.require=name=>{
 if(name==='node:child_process')return {execFileSync:(cli,args)=>{assert.ok(args[0]==='api'&&args[1].startsWith('query(')&&!args[1].includes('mutation'));return ${JSON.stringify(JSON.stringify(payload))};}};
 if(name==='./railway-browser-observation.cjs')return {...real,createObserver:options=>real.createObserver({...options,fetchImpl:async(url,settings)=>{
  assert.equal(settings.method,'GET');const u=new URL(url);
  if(globalThis.finalProbeStage&&!injected&&u.pathname==='/api/console/agents'){injected=true;return fetch('http://127.0.0.1:'+${fault.address().port}+'/',settings);}
  return fetch(${JSON.stringify(browserOrigin)}+u.pathname,{...settings,headers:{...settings.headers,'x-test-acceptance-host':u.hostname}});
 }})};
 return native(name);
};
moduleObject._compile(source,filename);
`;
  const runner=path.join(out,'loopback-runner.cjs');fs.writeFileSync(runner,prelude,{mode:0o600});
  child=spawn(process.execPath,[runner,path.join(APP,'web/node_modules/playwright'),'unused',out],{
    cwd:APP,env:{PATH:'/usr/bin:/bin',LC_ALL:'C'},stdio:['ignore','pipe','pipe']});
  let output='';child.stdout.on('data',d=>{output+=d;if(output.length>1048576)child.kill();});
  // Never print raw browser/process stderr, which may include private configuration.
  child.stderr.resume();
  const timer=setTimeout(()=>child.kill('SIGTERM'),240000);
  const exit=await new Promise(r=>child.once('exit',r));clearTimeout(timer);
  const report=JSON.parse(output.trim().split('\n').at(-1));
  fs.writeFileSync(path.join(out,'integration-result.json'),JSON.stringify(report,null,2));
  assert.equal(exit,0);assert.equal(report.passed,true);assert.equal(report.contexts.length,2);
  assert.ok(report.contexts.every(c=>c.passed&&c.my_agents_authentication&&c.factory_flow));
  assert.deepEqual(report.protected_route_acceptance,{passed:true,checks:19,read_only:true});
  assert.equal(report.private_api_requests,0);assert.equal(report.mutations,0);
  const audit=JSON.parse(fs.readFileSync(path.join(out,'browser-observations.json')));
  assert.ok(audit.some(r=>r.path==='/api/console/agents'&&r.state==='unavailable'&&r.error_type==='TypeError'&&r.transport_code==='UND_ERR_SOCKET'));
  assert.ok(audit.some(r=>r.path==='/api/console/agents'&&r.state==='pass'&&r.attempt>1));
  console.log(JSON.stringify({passed:true,actual_live_browser_entrypoint:true,actual_next_routes:true,
    desktop_mobile:true,protected_checks:19,native_fetch_socket_failure_recovered:true,
    public_private_api_calls:0,writes:0,railway_calls:0,production_images_built:0}));
 }catch(e){console.log(JSON.stringify({passed:false,error_type:e.name,assertion:e instanceof assert.AssertionError?'integration_assertion_failed':'integration_execution_failed'}));process.exitCode=1;}
 finally{
  if(child&&child.exitCode===null)child.kill('SIGTERM');
  await close(adapter);await close(fault);
  if(next&&next.exitCode===null){process.kill(-next.pid,'SIGTERM');await new Promise(r=>next.once('exit',r));}
 }
})();
