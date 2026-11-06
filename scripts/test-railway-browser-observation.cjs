'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {createServer}=require('node:http');
const {createObserver,PRIVATE_PATHS}=require('./railway-browser-observation.cjs');
async function fixture(handler,run) {
  const requests=[];
  const server=createServer((req,res)=>{requests.push({method:req.method,path:req.url});handler(req,res,requests.length);});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const fetchImpl=(url,options)=>fetch('http://127.0.0.1:'+server.address().port+new URL(url).pathname,
    {...options,headers:{...options.headers,'x-test-host':new URL(url).hostname}});
  const rows=[];
  const observer=createObserver({fetchImpl,deadline:performance.now()+2000,
    retryDelayMs:10,requestTimeoutMs:200,record:row=>rows.push(row)});
  try {await run({observer,rows,requests,fetchImpl});}
  finally {server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
}
const deny=(req,res)=>res.writeHead(req.headers.origin?403:401).end('Synthetic denial');
test('native fetch socket TypeError is diagnosed and recovered, then all 19 checks complete',async()=>{
  await fixture((req,res,n)=>n<=2?req.socket.destroy():deny(req,res),async({observer,rows,requests})=>{
    const proof=await observer.protectedRoutes();assert.deepEqual(proof,{passed:true,checks:19,read_only:true});
    assert.ok(rows.some(x=>x.state==='unavailable'&&x.error_type==='TypeError'&&x.transport_code==='UND_ERR_SOCKET'));
    assert.ok(rows.some(x=>x.path==='/my-agents'&&x.attempt>1&&x.state==='pass'));
    assert.equal(rows.filter(x=>x.state==='pass').length,19);
    assert.ok(requests.every(x=>x.method==='GET'));assert.equal(observer.acceptance,proof);
  });
});
test('persistent socket failures exhaust the shared deadline and cannot produce acceptance',async()=>{
  await fixture(req=>req.socket.destroy(),async({fetchImpl,rows})=>{
    const start=performance.now();const observer=createObserver({fetchImpl,deadline:start+160,
      retryDelayMs:10,requestTimeoutMs:50,record:r=>rows.push(r)});
    await assert.rejects(observer.protectedRoutes(),{code:'browser_observation_deadline_exhausted'});
    assert.equal(observer.acceptance,null);assert.ok(performance.now()-start>=140);
    assert.equal(rows.at(-1).reason,'browser_observation_deadline_exhausted');
  });
});
test('body timeout retries inside the original budget and closes hung reads',async()=>{
  await fixture((req,res)=>{res.writeHead(401);res.write('waiting');},async({fetchImpl,rows})=>{
    const observer=createObserver({fetchImpl,deadline:performance.now()+160,
      retryDelayMs:10,requestTimeoutMs:35,record:r=>rows.push(r)});
    await assert.rejects(observer.protectedRoutes(),{code:'browser_observation_deadline_exhausted'});
    assert.ok(rows.some(r=>r.reason==='request_timeout'));assert.equal(observer.acceptance,null);
  });
});
test('429 and 503 are unavailable observations, not successful denials',async()=>{
  await fixture((req,res,n)=>n<3?res.writeHead(n===1?429:503).end('retry'):deny(req,res),async({observer,rows})=>{
    assert.equal(await observer.status('/operator',{},401),401);
    assert.equal(rows.filter(r=>r.state==='unavailable').length,2);
    assert.equal(rows.at(-1).state,'pass');
  });
});
test('confirmed exposed private route fails immediately and never reads its body',async()=>{
  let calls=0,reads=0,cancelled=0;const rows=[];
  const observer=createObserver({record:r=>rows.push(r),fetchImpl:async()=>{
    calls++;return {status:200,body:{getReader(){reads++;throw Error('private-body');},cancel:async()=>{cancelled++;}}};
  }});
  await assert.rejects(observer.protectedRoutes(),{code:'security_boundary_violation'});
  assert.equal(calls,1);assert.equal(reads,0);assert.equal(cancelled,1);assert.equal(observer.acceptance,null);
  assert.equal(rows[0].observed_status,200);assert.equal(rows[0].state,'failed');
  assert.ok(!JSON.stringify(rows).includes('private-body'));
});
test('redirects are not followed or accepted as private-route protection',async()=>{
  await fixture((req,res)=>res.writeHead(302,{Location:'https://untrusted.invalid/secret'}).end(),async({observer,requests})=>{
    await assert.rejects(observer.status('/operator',{},401),{code:'protected_status_mismatch'});
    assert.equal(requests.length,1);
  });
});
test('cross-origin 200 is an immediate violation and invalid auth does not retry',async()=>{
  for(const [path,headers,expected,status,code] of [
    ['/api/operator-status',{origin:'https://untrusted.invalid'},403,200,'security_boundary_violation'],
    ['/my-agents',{authorization:'Basic dGVzdDp0ZXN0'},200,401,'authenticated_readiness_failed']]){
    let calls=0;const observer=createObserver({fetchImpl:async()=>{calls++;return new Response('',{status});}});
    await assert.rejects(observer.status(path,headers,expected),{code});assert.equal(calls,1);
  }
});
test('TLS errors and implementation TypeErrors are not mistaken for transient networking',async()=>{
  for(const error of [Object.assign(new TypeError('fetch failed'),{cause:{code:'CERT_HAS_EXPIRED'}}),new TypeError('private-implementation-detail')]){
    const rows=[];let calls=0;const observer=createObserver({record:r=>rows.push(r),fetchImpl:async()=>{calls++;throw error;}});
    await assert.rejects(observer.status('/operator',{},401),{code:error.cause?'tls_verification_failed':'probe_implementation_error'});
    assert.equal(calls,1);assert.ok(!JSON.stringify(rows).includes('private-implementation-detail'));
  }
});
test('diagnostics have operation, timing and fresh status without headers, bodies or raw errors',async()=>{
  let n=0;const rows=[];const observer=createObserver({record:r=>rows.push(r),retryDelayMs:1,fetchImpl:async()=>{
    if(!n++)throw Object.assign(new TypeError('fetch failed'),{cause:{code:'ECONNRESET',message:'private-secret'}});
    return new Response('private-response',{status:401});
  }});
  await observer.status('/operator',{authorization:'Basic cHJpdmF0ZTpwd2Q='},401);
  assert.equal(rows[0].observed_status,undefined);assert.equal(rows[0].operation,'protected_route');
  assert.equal(rows[0].path,'/operator');assert.equal(rows[0].error_type,'TypeError');
  assert.ok(rows.every(r=>Number.isFinite(r.elapsed_ms)&&Number.isFinite(r.remaining_ms)));
  assert.ok(!/private-secret|private-response|cHJpdmF0ZTpwd2Q=/.test(JSON.stringify(rows)));
});
test('successful observations consume bounded bodies; excessive bodies fail closed',async()=>{
  const response=new Response('small',{status:401});
  const observer=createObserver({fetchImpl:async()=>response});
  assert.equal(await observer.status('/operator',{},401),401);assert.equal(response.bodyUsed,true);
  const oversized=createObserver({fetchImpl:async()=>new Response('x'.repeat(65537),{status:401})});
  await assert.rejects(oversized.status('/operator',{},401),{code:'bounded_response_required'});
});
test('a late response cannot pass after the original deadline',async()=>{
  let clock=0;
  const observer=createObserver({now:()=>clock,deadline:100,fetchImpl:async()=>{clock=101;return new Response('',{status:401});}});
  await assert.rejects(observer.protectedRoutes(),{code:'browser_observation_deadline_exhausted'});
  assert.equal(observer.acceptance,null);
});
test('a transport ignoring abort is still bounded and its late body is disposed',async()=>{
  let cancelled=0,reads=0;const rows=[];
  const observer=createObserver({deadline:performance.now()+45,requestTimeoutMs:15,retryDelayMs:5,
    record:r=>rows.push(r),fetchImpl:async()=>{
      await new Promise(r=>setTimeout(r,80));
      return {status:401,body:{cancel:async()=>{cancelled++;},getReader(){reads++;throw Error('late-body');}}};
    }});
  await assert.rejects(observer.protectedRoutes(),{code:'browser_observation_deadline_exhausted'});
  await new Promise(r=>setTimeout(r,100));
  assert.ok(cancelled>0);assert.equal(reads,0);assert.equal(observer.acceptance,null);
  assert.equal(rows.filter(r=>r.state==='pass').length,0);
});
test('malformed responses cannot be mistaken for confirmed unauthorized access',async()=>{
  const rows=[];const observer=createObserver({record:r=>rows.push(r),fetchImpl:async()=>({status:'200'})});
  await assert.rejects(observer.status('/operator',{},401),{code:'invalid_observation_response'});
  assert.equal(rows[0].reason,'invalid_observation_response');
});
test('remaining time limits each request, and a non-advancing clock has a finite read bound',async()=>{
  let calls=0;const rows=[];
  const observer=createObserver({deadline:3,now:()=>0,retryDelayMs:1,sleep:async()=>{},record:r=>rows.push(r),
    fetchImpl:async()=>{calls++;throw Object.assign(new TypeError('fetch failed'),{cause:{code:'ECONNRESET'}});}});
  await assert.rejects(observer.status('/operator',{},401),{code:'browser_observation_deadline_exhausted'});
  assert.equal(calls,4);assert.ok(rows.filter(r=>r.timeout_ms).every(r=>r.timeout_ms<=3));
});
test('unknown targets, headers, expected states and invalid budgets never call the network',async()=>{
  let calls=0;const observer=createObserver({fetchImpl:async()=>{calls++;throw Error('must-not-run');}});
  for(const args of [['https://untrusted.invalid/',{},401],['/operator?secret=1',{},401],['/operator',{},undefined],
    ['/operator',{host:'untrusted.invalid'},401],['/operator',{'x-secret':'private'},401]])
    await assert.rejects(observer.status(...args));
  assert.equal(calls,0);
  for(const options of [{deadline:0},{requestTimeoutMs:0},{retryDelayMs:0},{requestTimeoutMs:15001}])
    assert.throws(()=>createObserver(options));
  assert.equal(PRIVATE_PATHS.length,17);
});
