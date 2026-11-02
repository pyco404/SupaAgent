/** Private disposable launch orchestration. Existing Rust endpoints enforce ownership. */
import {createHash} from 'node:crypto';
import {readFile,stat,mkdir,open,writeFile} from 'node:fs/promises';
import {dirname,resolve} from 'node:path';
import {operatorToken,localRequest} from '../web/src/lib/auth.ts';
const fail=(error,status=403,extra={})=>({status,body:{error,...extra}});
const hash=value=>createHash('sha256').update(value).digest('hex');
const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
async function scope(){
  if(process.env.SUPA_SITE_MODE==='staging')throw Error();
  const path=resolve(process.env.SUPA_LOCAL_LAUNCH_SCOPE??'/absent-supa-launch-scope');
  const info=await stat(path);if(!info.isFile()||info.uid!==process.getuid()||(info.mode&0o077))throw Error();
  const s=JSON.parse(await readFile(path,'utf8'));
  if(s.purpose!=='disposable_local_agent_launch'||s.primary_mutations_authorized!==false||s.expires_at<=Date.now()||s.backend_origin!==process.env.SUPA_BACKEND_URL||!/^supa_launch_[a-f0-9]{12}_test$/.test(s.database))throw Error();
  const base=new URL(s.backend_origin);if(base.protocol!=='http:'||base.hostname!=='127.0.0.1'||base.pathname!=='/'||base.username||base.password||base.search||base.hash)throw Error();
  return {s,base,directory:resolve(dirname(path),'launch-requests')};
}
export async function launch(path,headers,body,fetcher=fetch){
  if(path!=='/api/launch')return fail('write_not_permitted');
  const request=new Request('http://127.0.0.1'+path,{headers}),token=operatorToken(request.headers.get('authorization'));
  if(!localRequest(request))return fail('local_access_only');
  if(!token)return fail('operator_authentication_required',401);
  let context;try{context=await scope();}catch{return fail('disposable_launch_unavailable',503);}
  const {s,base,directory}=context;
  async function api(route,input){
    const r=await fetcher(new URL(route,base),{method:input===undefined?'GET':'POST',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},...(input===undefined?{}:{body:JSON.stringify(input)}),redirect:'error',cache:'no-store',signal:AbortSignal.timeout(7000)});
    if(!r.ok){const error=Error();error.status=[401,403,409].includes(r.status)?r.status:502;throw error;}
    const text=await r.text();if(text.length>200000)throw Error();return JSON.parse(text);
  }
  let status;try{status=await api('/operator-status');if(status.database!==s.database||status.epoch!==null)throw Error();}catch{return fail('disposable_database_not_verified',503);}
  if(body===undefined)return {status:200,body:{mode:'disposable_offline',enabled:status.role==='owner',role:status.role,expires_at:new Date(s.expires_at).toISOString(),paid_requests_enabled:false}};
  if(status.role!=='owner')return fail('owner_required');
  if(!request.headers.get('origin')||request.headers.get('x-supa-action')!=='offline-launch'||request.headers.get('content-type')?.split(';')[0]!=='application/json')return fail('explicit_same_origin_consent_required');
  let input;try{
    input=JSON.parse(body);if(Buffer.byteLength(body)>4096||Object.keys(input).sort().join(',')!=='authorize_offline,goal,name,personality,request_id,simulated_budget_microusd'||input.authorize_offline!==true||!uuid.test(input.request_id)||!Number.isSafeInteger(input.simulated_budget_microusd)||input.simulated_budget_microusd<3300||input.simulated_budget_microusd>1000000)throw Error();
    for(const [key,max]of [['name',80],['personality',500],['goal',500]])if(typeof input[key]!=='string'||input[key].trim().length<3||input[key].length>max||/[\x00-\x08\x0b-\x1f]/.test(input[key]))throw Error();
  }catch{return fail('invalid_launch_request',400);}
  const owner_hash=hash(token),file=resolve(directory,input.request_id+'.json');let record;
  try{
    await mkdir(directory,{mode:0o700});
  }catch(error){if(error.code!=='EEXIST')return fail('launch_journal_unavailable',503);}
  try{const handle=await open(file,'wx',0o600);record={request_id:input.request_id,owner_hash,input_sha256:hash(body),stage:'creating',agent_id:null};try{await handle.writeFile(JSON.stringify(record));await handle.sync();}finally{await handle.close();}}
  catch(error){if(error.code==='EEXIST'){try{const saved=JSON.parse(await readFile(file,'utf8'));return fail('launch_request_already_recorded',409,saved.owner_hash===owner_hash?{agent_id:saved.agent_id,stage:saved.stage}:{});}catch{}}return fail('launch_outcome_requires_inspection',409);}
  async function save(stage){record.stage=stage;await writeFile(file,JSON.stringify(record),{mode:0o600});}
  try{
    const agent=await api('/agents',{name:input.name.trim(),personality:{traits:['curious','transparent about mistakes'],voice:input.personality.trim()},config:{scheduler_enabled:false,retry_budget:0,max_steps:3,max_provider_calls:1,timeout_seconds:5,interval_seconds:1,inference_cost_microusd:1000,operating_cost_microusd:100},capabilities:['read_memory','remember','evaluate_goal']});
    if(!uuid.test(agent.id))throw Error();record.agent_id=agent.id;await save('adding_goal');
    await api(`/agents/${agent.id}/goals`,{description:input.goal.trim()});await save('assigning_simulated_seed');
    await api(`/agents/${agent.id}/simulated-funding`,{kind:'seed',amount_microusd:input.simulated_budget_microusd,idempotency_key:input.request_id,simulated:true});await save('created_inactive');
    return {status:200,body:{agent_id:agent.id,request_id:input.request_id,mode:'offline',lifecycle:agent.lifecycle,simulated_budget_microusd:input.simulated_budget_microusd,paid_requests:0}};
  }catch(error){return fail('launch_partial_or_unknown_inspect_saved_agent',error.status??503,{agent_id:record.agent_id,stage:record.stage});}
}
