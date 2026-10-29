/** Supa's read-only terminal views. All record text is inserted as text, never HTML. */
export const navigation = [
  ['home','Home'],['agents','Agents'],['intelligence','Intelligence'],['memory','Memory'],['economy','Economy'],['social','Social'],['trading','Trading · planned'],['token-launch','Token Launch · planned'],['factory','Factory · planned'],['about','About'],
].map(([id,label])=>({id,label,kind:'section',thumb:id==='home'?'/public/assets/supa-agent-code.svg':undefined,preview:id==='home'?undefined:id}));
export const description = 'Independent AI agents that earn money, pay for infrastructure, improve their capabilities, and create other agents on Solana.';
export const units = (value, decimals=6) => {
  if (!Number.isSafeInteger(value)) return 'unavailable';
  const amount=BigInt(value),scale=10n**BigInt(decimals),abs=amount<0n?-amount:amount;
  const fraction=(abs%scale).toString().padStart(decimals,'0').replace(/0+$/,'');
  return `${amount<0n?'−':''}${abs/scale}${fraction?'.'+fraction:''}`;
};
export async function read(path) {
  try {
    const response=await fetch(path,{cache:'no-store',credentials:'same-origin',signal:AbortSignal.timeout(6500)});
    if(!response.ok) return {error:response.status===401?'Operator sign-in required. Private records are not public.':response.status===404?'No saved resource is available.':'Saved records are unavailable. Check the local backend and reload.'};
    return {data:await response.json()};
  } catch { return {error:'Disconnected. Saved records could not be retrieved. Reload when the local backend is available.'}; }
}
export async function overview() {
  const [status,agents]=await Promise.all([read('/api/status'),read('/api/agents')]);
  return {status,agents,retrievedAt:new Date().toISOString()};
}
export async function sectionView(id, state, agentId) {
  const rows=[];
  const add=(text,kind='text')=>rows.push({text,kind});
  const record=(label,value)=>add(`${label}  ${value ?? 'unavailable'}`);
  const empty=label=>add(`No saved ${label} were returned.`);
  const load=async name=>{
    const result=await read(`/api/agents/${agentId}${name?'/'+name:''}`);
    if(result.error)add(`${name}: ${result.error}`,'notice');
    return result.data;
  };
  const agents=state.agents.data;
  add(`supa ${id==='home'?'status':id}`,'command');
  const epoch=state.status.data?.epoch;
  add(epoch?`Development epoch / ${epoch.id}. Prepared, read-only; historical database unrecovered.`:state.status.data?'Epoch unidentified. Historical recovery is not verified.':'Epoch unavailable while the backend is disconnected.','notice');
  if(id==='home') {
    add('Supa Agent / SUPA','heading');add('Born to think. Built to survive.','heading');
    add('Persistent AI agents with bounded reasoning, memory and economic awareness. Explore saved records; this site cannot start paid operations.');
    rows.push({kind:'navigate',id:'agents',text:'Explore saved agents →'});
    if(state.agents.error)add(state.agents.error,'notice');
    record('API',state.status.data?.status ?? 'disconnected');record('Economic mode','simulation');
    record('Agents',agents?.length);record('Retrieved',state.retrievedAt);
    add('Choose an identity in Agents, then explore Intelligence, Memory, Economy or Social. Private records require operator sign-in.','notice');
    add('Platform vision');add(description);add('Earning, infrastructure payments, capability improvement and agent creation describe the vision; autonomous real-money services are not enabled.');
  } else if(id==='agents') {
    add('Existing agents','heading');
    if(!agents) add(state.agents.error,'notice');
    else if(!agents.length)empty('agents');
    else for(const agent of agents) rows.push({kind:'agent',agent,text:`${agent.name} / ${agent.lifecycle}`});
    add('Choose an agent to inspect its saved records. Network and activity counts appear only when authorized records are available.');
  } else if(['trading','token-launch','factory'].includes(id)) {
    add('Planned / non-executable','heading');
    add(id==='trading'?'Trading is not enabled. Planned policy: a strict maximum of $5 per trade, subject to independent authorization and financial controls.':id==='token-launch'?'Pump.fun token launching is planned. No token has been launched through this interface.':'Human and agent-created agents are planned. Creation requires explicit authorization; this interface cannot create agents.');
    add('No action on this page signs, transfers, trades, publishes or launches anything.');
  } else if(id==='about') {
    add('Supa Agent / SUPA','heading');add('Born to think. Built to survive.');add('Vision: '+description);
    add('Current foundation: six-crate Rust runtime, PostgreSQL persistence, metered LLM abstraction, permissioned tools, simulated economics and read-only Devnet observations.');
    add('The model proposes actions. Application policy decides what is permitted. Personality never grants financial permissions.');
    add('Mainnet spending, trading, publishing, token launches and child-agent creation remain disabled. SUPA is brand notation, not evidence of a launched token.');
    rows.push({kind:'link',text:'Source / github.com/pyco404/SupaAgent',href:'https://github.com/pyco404/SupaAgent'});
  } else if(!agentId) { add('Select an agent in Agents to explore saved records.','notice'); }
  else {
    const agent=agents?.find(a=>a.id===agentId);
    add(`${agent?.name ?? 'Selected agent'} / ${agent?.lifecycle ?? 'status unavailable'}`,'heading');
    record('Identity',agentId);
    if(id==='intelligence') {
      add('Offline cycles and controlled live GPT sessions have separate histories and accounting. Viewing records never runs inference.','notice');
      const [profile,wallet,goals,cycles,actions,activity]=await Promise.all(['','wallet','goals','cycles','actions','activity'].map(load));
      const [schedule,health,history,access]=await Promise.all(['schedule','execution-health','authorizations','access'].map(resource=>read(`/api/agents/${agentId}/${resource}`)));
      if(access.data)record('Your role',access.data.role);
      const live=await read(`/api/agents/${agentId}/live`);
      if(live.data && access.data)rows.push({kind:'live',view:live.data,role:access.data.role,agentId});
      else add('Controlled live inference unavailable. No paid requests are enabled by viewing this page.','notice');
      if(health.data){record('Worker health',health.data.status.replaceAll('_',' '));record('Worker process',health.data.worker_enabled===false?'Scheduling disabled in observed process':health.data.worker_enabled===true?'Scheduling enabled in observed process':'Configuration not reported');record('Execution',health.data.execution_running?'Measured active lease and recent heartbeat':'No verified running execution');record('Heartbeat observed',health.data.heartbeat_at??'No heartbeat recorded');record('Lease expires',health.data.lease_until??'No active lease');add('Authorization permits work; it does not prove a worker is running.','notice');}
      else add('Worker health is unavailable. No running execution can be verified.','notice');
      if(schedule.data && goals && access.data)rows.push({kind:'schedule',status:schedule.data,goals,role:access.data.role});
      if(history.data?.length)rows.push({kind:'detail',text:'Authorization approval history',body:history.data.map(a=>`Authorization ${a.id} / ${a.legacy?'legacy: approver not recorded':a.approved_by??'approver unavailable'}\nApproved ${a.approved_at??'time not recorded'} / expires ${a.expires_at} / closed ${a.closed_at??'current epoch'}\nGoals ${a.policy.goal_ids.join(', ')}\nLimit ${a.policy.max_cycles} cycles / ${a.policy.simulated_budget_microusd} simulated microUSD / interval ${a.policy.interval_seconds}s\nConsumed ${a.cycles_started} cycles / ${a.reserved_microusd} simulated microUSD`).join('\n\n')});
      if(profile){add('Agent profile','heading');record('Lifecycle',profile.agent.lifecycle);record('Created',profile.agent.created_at);record('Verified activity records',activity?.length);record('Wallet network',wallet?.network ?? 'No saved wallet identity');record('Wallet address',wallet?.address ?? 'unavailable');}
      if(goals){add('Goals','heading');if(!goals.length)empty('goals');goals.forEach(g=>add(`${g.completed?'complete':'open'} / priority ${g.priority} / suggested progress ${g.suggested_progress_bps/100}%\n${g.description}\nref ${g.id}`));}
      if(cycles){add(`Saved reasoning cycles / ${cycles.length}`,'heading');cycles.forEach(c=>add(`${c.status} / ${c.started_at}\ncycle ${c.id}\nfinished ${c.finished_at ?? 'not recorded'}${c.failure_code?' / failure '+c.failure_code:''}`));}
      if(actions){add(`Tool actions / ${actions.length}`,'heading');actions.forEach(a=>add(`${a.proposal.tool} / authorization ${a.authorization_outcome} / ${a.status}\ncycle ${a.cycle_id}${a.failure_code?' / failure '+a.failure_code:''}`));}
      if(activity){add(`Saved activity / ${activity.length}`,'heading');activity.forEach(e=>add(`${e.simulated?'Simulated':'Recorded'} / ${e.kind} / ${e.code}\n${e.created_at} / event ${e.id}`));}
    } else if(id==='memory') {
      add('Operator-only learning records. Hypotheses are not established facts.','notice');
      const memories=await load('memory');
      if(memories){if(!memories.length)empty('memories');memories.forEach(m=>add(`${m.kind==='verified_observation'?'Verified observation':'Hypothesis / unverified'}\n${m.lesson}\n${m.created_at}\nevents ${m.source_event_ids.join(', ') || 'none'}\ncycle ${m.cycle_id ?? 'not recorded'}`));}
    } else if(id==='economy') {
      const [financials,ledger,wallet,balances,treasury,payments]=await Promise.all(['financials','simulated-ledger','wallet','wallet/balances','wallet/treasury','wallet/payments'].map(load));
      add('Simulated ledger / hypothetical USD','heading');
      if(financials){for(const [key,label]of [['balance_microusd','Balance'],['revenue_microusd','Revenue'],['inference_microusd','Inference expense'],['operating_microusd','Operating expense']])record(`${label} / simulated USD`,units(financials[key]));record('Runway',financials.runway_days===null?'Unavailable: a usable recurring-cost estimate is not established.':`${financials.runway_days} days (estimate)`);rows.push({kind:'detail',text:'Runway calculation',body:financials.runway_assumption});}
      if(ledger){if(!ledger.length)empty('simulated transactions');ledger.forEach(e=>add(`Simulated / ${e.kind} / ${units(e.amount_microusd)} USD\n${e.created_at} / ${e.id}`));}
      add('Solana Devnet / test assets only','heading');
      if(wallet)record('Wallet / Devnet',wallet.address);
      if(balances){record('Evidence',balances.evidence_source==='configured_rpc'?'Configured RPC observation (saved)':'Mock RPC / test fixture');record('SOL / Devnet',units(balances.sol_lamports,9));record('USDC / Devnet',units(balances.usdc_atoms));record('Observed slot',balances.slot);}
      else if(balances===null)add('No saved wallet balance observation.');
      if(treasury){record('Observation time',treasury.observed_at);record('Observation freshness',treasury.fresh?'fresh per backend':'stale');record('Reconciled',treasury.reconciled?'yes':'no');record('Verified incoming USDC / Devnet',units(treasury.verified_incoming_usdc_atoms));}
      if(payments){add('Recorded confirmed receipts / Devnet','heading');if(!payments.length)empty('receipts');payments.forEach(p=>add(`Devnet / ${p.evidence_source} / ${units(p.amount_atoms)} USDC\nmint ${p.mint}\nsignature ${p.signature}\nslot ${p.slot} / confirmed receipt record`));}
      add('Mainnet / unavailable. Devnet assets and simulated USD are separate and have no real-money value here. Signing is disabled.','notice');
    } else if(id==='social') {
      const [social,drafts]=await Promise.all(['social','drafts'].map(load));
      add('Persisted drafts / never published by this website','heading');
      add('Original stored wording follows. Financial statements concern simulation unless the cited record explicitly identifies Devnet test assets. Provider attribution is unavailable in this read API.','notice');
      if(drafts){if(!drafts.length)empty('drafts');drafts.forEach(d=>rows.push({kind:'draft',text:d.data.text ?? 'No draft text / abstention',meta:`Unpublished / ${d.data.style ?? 'style not recorded'} / ${d.created_at}\nevents ${d.data.event_ids.join(', ') || 'none'}\nmemories ${d.data.memory_ids?.join(', ') || 'none'}\ndraft ${d.id}`}));}
      if(social){add('Review history','heading');if(!social.length)empty('review records');social.forEach(s=>add(`${s.disposition} / ${s.data.choice ?? 'choice not recorded'} / ${s.data.style ?? 'style not recorded'}\n${s.data.reason ?? s.failure_code ?? 'No review reason saved'}\nevents ${s.data.event_ids.join(', ') || 'none'}\n${s.created_at}`));}
      add('X publishing / disabled. No publication history endpoint is available.');
    }
    record('Retrieved',new Date().toISOString());
  }
  if(id==='intelligence' && epoch?.state==='prepared')add('Prepared epoch: ownership changes, authorizations and execution controls are disabled. No activation has been approved.','notice');
  if(id==='intelligence' && epoch?.state!=='prepared'){
    const workflow=await read('/api/autonomy');
    if(workflow.data)rows.splice(1,0,{kind:'autonomy',policy:workflow.data});
    else add('Offline autonomy controls unavailable. Operator sign-in and a backend supporting this workflow are required.','notice');
  }
  return rows;
}
