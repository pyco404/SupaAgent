'use strict';
// Read-only deployment acceptance. No Railway mutations or credential retrieval.
const ORIGINS = Object.freeze({apex:'https://supaagency.xyz', staging:'https://supa-web-staging.up.railway.app'});
const PRIVATE_PATHS = Object.freeze(['/my-agents','/my-agents/private','/operator','/api/agents',
  '/api/operator-status','/api/console/agents','/agents/new',
  '/agents/11111111-1111-4111-8111-111111111111','/memory/private','/economy/wallet',
  '/token-launch/create','/factory/create','/social/drafts','/trading/orders',
  '/intelligence/control','/wallets','/admin']);
const PATHS = new Set(['/','/healthz',...PRIVATE_PATHS]);
const OPERATIONS = new Set(['readiness','anonymous_management','forged_management',
  'authenticated_management','protected_route','staging_authentication','cross_origin']);
const RETRY_STATUS = new Set([429,500,502,503,504]);
const NETWORK_CODES = new Set(['ECONNRESET','ECONNREFUSED','ENOTFOUND','EAI_AGAIN','ETIMEDOUT',
  'ENETUNREACH','EHOSTUNREACH','UND_ERR_SOCKET','UND_ERR_CONNECT_TIMEOUT',
  'UND_ERR_HEADERS_TIMEOUT','UND_ERR_BODY_TIMEOUT']);
const TLS_CODES = new Set(['CERT_HAS_EXPIRED','ERR_TLS_CERT_ALTNAME_INVALID',
  'DEPTH_ZERO_SELF_SIGNED_CERT','UNABLE_TO_VERIFY_LEAF_SIGNATURE','SELF_SIGNED_CERT_IN_CHAIN']);
class ProbeFailure extends Error {
  constructor(code) { super(code); this.name='ProbeFailure'; this.code=code; }
}
function classify(error) {
  if (error instanceof ProbeFailure) return {retry:false,reason:error.code,error_type:'ProbeFailure'};
  const code=error?.cause?.code ?? error?.code;
  const error_type=['TypeError','TimeoutError','AbortError','Error'].includes(error?.name)?error.name:'Error';
  const message_id=error?.message==='fetch failed'?'native_fetch_failed':error?.message==='terminated'?'response_terminated':'unretained';
  if (TLS_CODES.has(code)) return {retry:false,reason:'tls_verification_failed',error_type,message_id,transport_code:code};
  if (NETWORK_CODES.has(code)) return {retry:true,reason:'network_unavailable',error_type,message_id,transport_code:code};
  if (['TimeoutError','AbortError'].includes(error_type)) return {retry:true,reason:'request_timeout',error_type};
  // Native fetch's fixed error text is recognized, never copied into evidence.
  if (error_type==='TypeError' && error?.message==='fetch failed')
    return {retry:true,reason:'fetch_unavailable',error_type,message_id};
  return {retry:false,reason:'probe_implementation_error',error_type};
}
function createObserver({deadline=performance.now()+220000, requestTimeoutMs=10000,
  retryDelayMs=500, fetchImpl=globalThis.fetch, now=()=>performance.now(),
  sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms)), record=()=>{}}={}) {
  if (!Number.isFinite(deadline) || deadline<=now() || !Number.isFinite(requestTimeoutMs)
      || requestTimeoutMs<=0 || requestTimeoutMs>15000 || !Number.isFinite(retryDelayMs)
      || retryDelayMs<=0) throw new ProbeFailure('invalid_observation_bounds');
  let finalAcceptance=null;
  async function status(path, headers={}, expected, operation='protected_route') {
    if (!PATHS.has(path) || !OPERATIONS.has(operation) || ![200,401,403].includes(expected)
        || !headers || Object.getPrototypeOf(headers)!==Object.prototype)
      throw new ProbeFailure('approved_read_only_observation_required');
    const clean={}; let origin='apex';
    for (const [key,value] of Object.entries(headers)) {
      if (key==='host' && value==='supa-web-staging.up.railway.app') {origin='staging';continue;}
      if (!['authorization','origin','x-supa-public-landing','x-supa-experience'].includes(key)
          || typeof value!=='string' || value.length>4096
          || (key==='authorization' && !/^Basic [A-Za-z0-9+/=]+$/.test(value))
          || (key==='origin' && value!=='https://untrusted.invalid')
          || (key.startsWith('x-supa-') && value!=='1'))
        throw new ProbeFailure('approved_observation_headers_required');
      clean[key]=value;
    }
    const started=now();
    const limit=Math.ceil(Math.max(0,deadline-started)/retryDelayMs)+1;
    for (let attempt=1;attempt<=limit;attempt++) {
      const remaining=deadline-now();
      if (remaining<=0) break;
      const timeout=Math.max(1,Math.min(requestTimeoutMs,remaining));
      const audit={operation,origin,path,method:'GET',attempt,
        elapsed_ms:Math.max(0,Math.round(now()-started)),remaining_ms:Math.max(0,Math.round(remaining)),
        timeout_ms:Math.round(timeout),expected_status:expected};
      const abort=new AbortController(); let timer,response,reader;
      try {
        const task=(async()=>{
          response=await fetchImpl(ORIGINS[origin]+path,{method:'GET',headers:clean,
            credentials:'omit',redirect:'manual',signal:abort.signal});
          if (abort.signal.aborted) {void response.body?.cancel().catch(()=>{});throw new DOMException('','TimeoutError');}
          if (!Number.isInteger(response.status) || response.status<100 || response.status>599)
            throw new ProbeFailure('invalid_observation_response');
          audit.observed_status=response.status;
          // Reject a leaked private route before reading any private contents.
          if (response.status!==expected && !RETRY_STATUS.has(response.status))
            throw new ProbeFailure(expected===401 || expected===403
              ? (response.status>=200 && response.status<300?'security_boundary_violation':'protected_status_mismatch')
              : 'authenticated_readiness_failed');
          // Consume bounded bodies to release reusable fetch connections. No body is retained.
          let bytes=0;
          if (response.body) {
            reader=response.body.getReader();
            while (true) {
              const chunk=await reader.read(); if (chunk.done) break;
              bytes+=chunk.value.byteLength;
              if (bytes>65536) throw new ProbeFailure('bounded_response_required');
            }
          }
          if (RETRY_STATUS.has(response.status)) return false;
          return true;
        })();
        const completed=await Promise.race([task,new Promise((_,reject)=>{
          timer=setTimeout(()=>{abort.abort();reject(new DOMException('','TimeoutError'));},timeout);
        })]);
        if (now()>=deadline) throw new ProbeFailure('browser_observation_deadline_exhausted');
        if (completed) {
          record({...audit,state:'pass',elapsed_ms:Math.max(0,Math.round(now()-started))});
          return response.status;
        }
        record({...audit,state:'unavailable',reason:'upstream_unavailable'});
      } catch (error) {
        const info=classify(error);
        record({...audit,state:info.retry?'unavailable':'failed',...info,
          elapsed_ms:Math.max(0,Math.round(now()-started))});
        if (!info.retry) throw new ProbeFailure(info.reason);
      } finally {
        clearTimeout(timer); abort.abort();
        if (reader) {void reader.cancel().catch(()=>{});}
        else if (response?.body) {void response.body.cancel().catch(()=>{});}
      }
      const budget=deadline-now(); if (budget<=0) break;
      await sleep(Math.min(retryDelayMs,budget));
    }
    record({operation,origin,path,method:'GET',state:'failed',
      reason:'browser_observation_deadline_exhausted',elapsed_ms:Math.max(0,Math.round(now()-started))});
    throw new ProbeFailure('browser_observation_deadline_exhausted');
  }
  async function protectedRoutes() {
    finalAcceptance=null;
    for (const path of PRIVATE_PATHS) await status(path,{},401,'protected_route');
    await status('/',{host:'supa-web-staging.up.railway.app'},401,'staging_authentication');
    await status('/api/operator-status',{origin:'https://untrusted.invalid'},403,'cross_origin');
    finalAcceptance=Object.freeze({passed:true,checks:PRIVATE_PATHS.length+2,read_only:true});
    return finalAcceptance;
  }
  return {status,protectedRoutes,get acceptance(){return finalAcceptance;}};
}
module.exports={createObserver,ProbeFailure,PRIVATE_PATHS};
