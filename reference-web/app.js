// Supa Agent / SUPA
// Born to think. Built to survive.
// Persistent agents. Application-enforced permissions.
import { createWheel } from './wheel.js';
import { createScenes } from './scene.js';
import { navigation, overview, sectionView } from './platform.js';

const $ = selector => document.querySelector(selector);
const root = document.documentElement;
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
reduced.addEventListener('change', event => { root.dataset.motion=event.matches?'still':'running';if(event.matches)revealTyping?.(); });
root.dataset.motion = reduced.matches?'still':'running';
const node = (tag,className,text) => { const el=document.createElement(tag);if(className)el.className=className;if(text!==undefined)el.textContent=String(text);return el; };
const palettes=[['#0c0c0c','#d8d4b4','#949078'],['#0b0b0b','#ebe6db','#9b968a'],['#11120f','#d9d7c3','#9b9d85']];
let sceneData, revealTyping=null;
let theme=0,wheel,scenes,itemIndex=0,typing=0,typingTimer=0,leaving=false,entryAttempt=0,worm=false;
let state={status:{error:'Connecting to local backend…'},agents:{error:'Loading saved agents…'},retrievedAt:'not yet retrieved'},agentId='',renderVersion=0,reloadVersion=0;
let commandHistory=[],historyIndex=0,liveTimer=0,liveBusy=false,viewFingerprint='';
let autonomyAttempt=null;
try{const id=sessionStorage.getItem('supa.offline-request');if(/^[0-9a-f-]{36}$/i.test(id??''))autonomyAttempt={id,status:'unknown'};}catch{/* Optional recovery hint; server UUID uniqueness remains authoritative. */}
try{const saved=Number(localStorage.getItem('supa.dark-theme'));if(Number.isInteger(saved)&&saved>=0&&saved<palettes.length)theme=saved;}catch{/* Optional preference. */}
function applyTheme(){['--bg','--ink','--dim'].forEach((name,index)=>root.style.setProperty(name,palettes[theme][index]));}
applyTheme();
// Keep the native value as backing state; the visible picker never uses an OS popup.
function closeAgentPicker(focus=false){$('#agent-options').hidden=true;$('#agent-picker-trigger').setAttribute('aria-expanded','false');if(focus)$('#agent-picker-trigger').focus({preventScroll:true});}
function syncAgentPicker(){
  const native=$('#agent-select'),list=$('#agent-options'),trigger=$('#agent-picker-trigger'),restore=list.contains(document.activeElement);
  $('#agent-selected-name').textContent=native.selectedOptions[0]?.textContent ?? 'Choose an agent';trigger.title=$('#agent-selected-name').textContent;trigger.disabled=native.options.length<2;
  list.replaceChildren();
  for(const option of native.options){
    const button=node('button','agent-option',option.textContent);button.type='button';button.setAttribute('role','option');button.setAttribute('aria-selected',String(option.value===native.value));button.tabIndex=-1;
    button.addEventListener('click',()=>{native.value=option.value;native.dispatchEvent(new Event('change'));closeAgentPicker(true);});list.append(button);
  }
  closeAgentPicker(restore);
}
function openAgentPicker(last=false){
  const trigger=$('#agent-picker-trigger'),list=$('#agent-options');if(trigger.disabled)return;
  const box=trigger.getBoundingClientRect(),below=Math.max(0,innerHeight-box.bottom-12),above=Math.max(0,box.top-12),up=below<180&&above>below;
  list.style.width=`${Math.min(box.width,innerWidth-16)}px`;list.style.left=`${Math.max(8,Math.min(box.left,innerWidth-box.width-8))}px`;
  list.style.top=up?'auto':`${box.bottom+4}px`;list.style.bottom=up?`${innerHeight-box.top+4}px`:'auto';list.style.maxHeight=`${Math.min(240,up?above:below)}px`;
  list.hidden=false;trigger.setAttribute('aria-expanded','true');
  const options=[...list.children],active=last?options.at(-1):options.find(option=>option.getAttribute('aria-selected')==='true')??options[0];active?.focus({preventScroll:true});active?.scrollIntoView({block:'nearest'});
}
$('#agent-picker-trigger').addEventListener('click',()=>$('#agent-options').hidden?openAgentPicker():closeAgentPicker());
$('.agent-picker').addEventListener('keydown',event=>{
  if(event.ctrlKey||event.metaKey||event.altKey)return;
  const options=[...$('#agent-options').children],index=options.indexOf(document.activeElement);
  if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){
    event.preventDefault();event.stopPropagation();if($('#agent-options').hidden){openAgentPicker(event.key==='ArrowUp'||event.key==='End');return;}
    const next=event.key==='Home'?0:event.key==='End'?options.length-1:Math.max(0,Math.min(options.length-1,index+(event.key==='ArrowDown'?1:-1)));options[next]?.focus({preventScroll:true});options[next]?.scrollIntoView({block:'nearest'});
  }else if(event.key==='Escape'){event.preventDefault();event.stopPropagation();closeAgentPicker(true);}else if(event.key==='Tab')closeAgentPicker(true);else if(event.key.length===1&&event.key!==' '){event.preventDefault();event.stopPropagation();if($('#agent-options').hidden)openAgentPicker();options.find(option=>option.textContent.toLowerCase().startsWith(event.key.toLowerCase()))?.focus();}
});
document.addEventListener('pointerdown',event=>{if(!$('.agent-picker').contains(event.target))closeAgentPicker();});
window.addEventListener('resize',()=>closeAgentPicker($('#agent-options').contains(document.activeElement)));
document.addEventListener('scroll',event=>{const target=event.target;if(target===document||target instanceof Element&&target.contains($('#agent-picker-trigger')))closeAgentPicker($('#agent-options').contains(document.activeElement));},true);
const ready=boot();
async function boot(){
  const data=sceneData={days:[],account:'Supa Agent',connection:'connecting',agentCount:null,retrievedAt:null};
  scenes=createScenes({intro:$('.intro-canvas'),blueprint:$('.blueprint'),entity:$('.entity'),transition:$('.transition'),spark:$('.spark'),reduced,data});
  document.fonts.ready.then(()=>scenes?.redraw());
  fetch('/app.js').then(r=>r.ok?r.text():'').then(text=>{if(text)scenes?.setCode(text);}).catch(()=>{});
  wheel=createWheel({host:$('.wheel'),canvas:$('.wheel-ink'),items:navigation,reducedMotion:reduced,onSelect:index=>select(index)});
  const initial=location.hash;setView(location.pathname.startsWith('/site'));if(document.body.dataset.view==='projects')openHash(initial);
  root.dataset.ready='true';
  void reload(data);
  liveTimer=setInterval(()=>void refreshLive(),15000);
  return true;
}
// Read-only polling pauses while hidden, typing or interacting. It never starts a cycle.
async function refreshLive(){
  if(liveBusy||document.hidden||document.body.dataset.view!=='projects'||$('.buffer').getAttribute('aria-busy')==='true'||!$('.command').hidden||!$('#agent-options').hidden||$('.projects').classList.contains('drawer-open'))return;
  liveBusy=true;
  try{
    const version=reloadVersion,next=await overview();
    if(version!==reloadVersion||document.hidden||document.body.dataset.view!=='projects')return;
    const changed=JSON.stringify([state.status,state.agents])!==JSON.stringify([next.status,next.agents]);
    state=next;
    if(changed){await reload();return;}
    await select(itemIndex,true);
  }finally{liveBusy=false;}
}
async function reload(currentScene=sceneData){
  const version=++reloadVersion;$('.reload-records').disabled=true;$('.reload-records').textContent='Retrieving…';
  try {
    const next=await overview();if(version!==reloadVersion)return;state=next;
    if(!state.agents.data?.some(agent=>agent.id===agentId))agentId=state.agents.data?.[0]?.id ?? '';
    const selectEl=$('#agent-select');selectEl.replaceChildren(node('option','',state.agents.error?'Agent records unavailable':state.agents.data?.length?'Choose an agent':'No saved agents'));selectEl.firstChild.value='';
    for(const agent of state.agents.data ?? []){const option=node('option','',`${agent.name} / ${agent.lifecycle}`);option.value=agent.id;selectEl.append(option);}
    selectEl.value=agentId;syncAgentPicker();
    $('[data-stat=repos]').textContent=state.agents.data?.length ?? '—';
    $('[data-stat=stars]').textContent=state.status.data?.status ?? 'offline';
    $('[data-stat=followers]').textContent='SIM';
    $('.snapshot-date').textContent=state.retrievedAt.slice(11,19)+' UTC';$('.snapshot-date').title=state.retrievedAt;
    $('.bar').title=`Saved records retrieved ${state.retrievedAt}; not a blockchain observation timestamp.`;
    if(currentScene){currentScene.connection=state.status.data?.status ?? 'disconnected';currentScene.agentCount=state.agents.data?.length ?? null;currentScene.retrievedAt=state.retrievedAt;scenes.redraw();}
    $('.announcement').textContent=state.agents.error ?? 'Saved records retrieved.';
    if(document.body.dataset.view==='projects')void select(itemIndex);
  }finally{if(version===reloadVersion){$('.reload-records').disabled=false;$('.reload-records').textContent='Reload saved records';}}
}
$('.reload-records').addEventListener('click',()=>{closeCommand(false);$('.buffer').focus({preventScroll:true});void reload();});
$('#agent-select').addEventListener('change',event=>{agentId=event.target.value;syncAgentPicker();void select(itemIndex);});
function setView(projects){
  closeAgentPicker();
  entryAttempt++;leaving=false;scenes?.cancelWarp();cancelTyping();renderVersion++;$('.buffer').setAttribute('aria-busy','false');
  document.body.dataset.view=projects?'projects':'intro';$('.intro').hidden=projects;$('.projects').hidden=!projects;closeCommand(false);setDrawer(false,false);scenes?.setMode(projects?'projects':'intro');
  if(projects){void select(itemIndex);requestAnimationFrame(()=>{window.dispatchEvent(new Event('resize'));$('.buffer').focus({preventScroll:true});});}
}
async function enter(event){event?.preventDefault();if(!await ready||leaving||document.body.dataset.view!=='intro')return;leaving=true;const attempt=++entryAttempt,completed=await scenes.warp();if(!completed||attempt!==entryAttempt)return;history.pushState({view:'projects'},'','/site/');setView(true);}
$('.enter').addEventListener('click',enter);
$('.brand').addEventListener('click',async event=>{event.preventDefault();if(!await ready)return;history.pushState({view:'intro'},'','/');setView(false);$('.enter').focus({preventScroll:true});});
window.addEventListener('popstate',()=>{const hash=location.hash;setView(location.pathname.startsWith('/site'));if(document.body.dataset.view==='projects')openHash(hash);});
window.addEventListener('hashchange',()=>{if(root.dataset.ready==='true'&&document.body.dataset.view==='projects')openHash(location.hash);});
function openHash(hash){let id='home';try{id=decodeURIComponent(hash.replace(/^#/,''))||'home';}catch{/* Unknown fragments return Home. */}const index=navigation.findIndex(item=>item.id===id);wheel.select(Math.max(0,index));void select(Math.max(0,index));}
function changeTheme(){theme=(theme+1)%palettes.length;applyTheme();try{localStorage.setItem('supa.dark-theme',String(theme));}catch{/* Optional preference. */}scenes.redraw();window.dispatchEvent(new Event('resize'));$('.announcement').textContent=`Dark theme ${theme+1} of ${palettes.length}`;}
$('.theme-key').addEventListener('click',changeTheme);
function setDrawer(open,focus=true){
  $('.projects').classList.toggle('drawer-open',open);$('.tabline').setAttribute('aria-expanded',String(open));$('.menu-indicator').textContent=open?'×':'☰';
  $('.tabline').setAttribute('aria-label',open?'Close navigation':'Open navigation');
  document.body.classList.toggle('menu-open',open);
  $('.pane').inert=open;$('.bar').inert=open;
  if(focus)(open?$('.wheel'):$('.tabline')).focus({preventScroll:true});
  requestAnimationFrame(()=>window.dispatchEvent(new Event('resize')));
}
$('.tabline').addEventListener('click',()=>setDrawer($('.tabline').getAttribute('aria-expanded')!=='true'));
$('.wheel').addEventListener('click',event=>{if(event.target.closest('.wheel-item')&&matchMedia('(max-width:700px)').matches){setDrawer(false,false);$('.buffer').focus({preventScroll:true});}});
function line(text,className='terminal-line'){const el=node('p',className,text);$('.buffer').append(el);return el;}
async function select(index,background=false){
  const item=navigation[index];if(!item)return;
  const version=background?renderVersion:++renderVersion;
  if(!background){itemIndex=index;cancelTyping();}
  if(document.body.dataset.view==='projects')history.replaceState(null,'',`#${item.id}`);
  const buffer=$('.buffer');
  if(!background){
    $('.tab-file').textContent=item.label;$('.winbar').textContent=`supa@local — ${item.label} / read-only`;
    buffer.replaceChildren();buffer.setAttribute('aria-busy','true');buffer.dataset.typingState='loading';
    $('.blueprint').hidden=!['home','about'].includes(item.id);scenes?.setSelection(index);
    line('supa@local ~','prompt');line('Loading saved view…','prompt');
  }
  const rows=await sectionView(item.id,state,agentId);
  if(version!==renderVersion||document.body.dataset.view!=='projects')return;
  $('.winbar').textContent=`supa@local — ${item.label} / ${rows.some(row=>row.kind==='autonomy')?'offline controls':'read-only'}`;
  const fingerprint=JSON.stringify(rows.filter(row=>!row.text?.startsWith('Retrieved  ')));
  if(background&&(fingerprint===viewFingerprint||buffer.contains(document.activeElement)||!$('.command').hidden||buffer.getAttribute('aria-busy')==='true'))return;
  viewFingerprint=fingerprint;if(background)cancelTyping();
  buffer.replaceChildren();line('supa@local ~','prompt');
  for(const row of rows){
    if(row.kind==='agent'){
      const button=node('button','terminal-entry',row.text);button.addEventListener('click',()=>{agentId=row.agent.id;$('#agent-select').value=agentId;syncAgentPicker();wheel.select(2);void select(2);});buffer.append(button);line(`Identity ${row.agent.id}\nCreated ${row.agent.created_at}`,'prompt');
    }else if(row.kind==='autonomy')buffer.append(autonomyPanel(row.policy));
    else if(row.kind==='link'){const p=line('');const a=node('a','',row.text);a.href=row.href;a.target='_blank';a.rel='noopener noreferrer';p.append(a);}
    else if(row.kind==='detail'){const details=node('details','calculation');details.append(node('summary','',row.text),node('p','',row.body));buffer.append(details);}
    else if(row.kind==='draft'){const block=node('figure','saved-draft');block.append(node('blockquote','',row.text),node('figcaption','prompt',row.meta));buffer.append(block);}
    else if(row.kind==='navigate'){const button=node('button','terminal-entry',row.text);button.addEventListener('click',()=>{const target=navigation.findIndex(item=>item.id===row.id);wheel.select(target);void select(target);});buffer.append(button);}
    else if(row.kind==='heading')buffer.append(node('h2','terminal-heading',row.text));
    else line(row.text,row.kind==='command'?'terminal-command':row.kind==='heading'?'terminal-heading':row.kind==='notice'?'terminal-notice':'terminal-line');
  }
  buffer.scrollTop=0;$('.pane').classList.remove('is-changing');void buffer.offsetWidth;$('.pane').classList.add('is-changing');
  $('.announcement').textContent=`Selected ${item.label}`;$('.key-feedback').textContent=': commands · ↑↓ navigate';typeTerminal();
}
// Every visit reveals complete sections in document order, using saved text only.
function cancelTyping(){
  scenes?.stopTyping(true);
  typing++;clearTimeout(typingTimer);revealTyping=null;
  $('.skip-typing').hidden=true;
}
$('.skip-typing').addEventListener('click',()=>{revealTyping?.();closeCommand(false);});
function typeTerminal() {
  clearTimeout(typingTimer);
  const buffer=$('.buffer'),token=++typing,rows=[...buffer.children],skip=$('.skip-typing');
  const ghost=document.querySelectorAll('.entity-state b')[1];
  const originals=[];
  let rowIndex=0,textIndex=0,character=0,totalTyped=0;
  function finish(){
    if(token!==typing)return;
    clearTimeout(typingTimer);
    originals.forEach(({text,value})=>{text.textContent=value;});
    rows.forEach(row=>{row.hidden=false;row.inert=false;});
    buffer.setAttribute('aria-busy','false');buffer.setAttribute('aria-live','polite');buffer.dataset.typingState='complete';
    scenes?.stopTyping();
    if(ghost)ghost.textContent='idle';
    const restoreFocus=document.activeElement===skip;
    skip.hidden=true;revealTyping=null;
    if(restoreFocus)buffer.focus({preventScroll:true});
  }
  if(reduced.matches){finish();return;}
  rows.forEach(row=>{
    row.hidden=true;row.inert=true;
    const walker=document.createTreeWalker(row,NodeFilter.SHOW_TEXT);
    let text;while((text=walker.nextNode())){originals.push({text,value:text.textContent,row});text.textContent='';}
  });
  buffer.setAttribute('aria-busy','true');buffer.setAttribute('aria-live','off');buffer.dataset.typingState='typing';
  if(ghost)ghost.textContent='typing';
  skip.hidden=false;revealTyping=finish;
  function tick(){
    if(token!==typing)return;
    if(reduced.matches){finish();return;}
    const row=rows[rowIndex];if(!row){finish();return;}
    const follow=buffer.scrollHeight-buffer.scrollTop-buffer.clientHeight<32;
    row.hidden=false;
    const record=originals[textIndex];
    if(!record||record.row!==row){row.inert=false;rowIndex++;typingTimer=setTimeout(tick,120);return;}
    if(character<record.value.length){
      character++;record.text.textContent=record.value.slice(0,character);
      if(follow)buffer.scrollTop=buffer.scrollHeight;
      scenes?.onTyping(record.text,character,record.value[character-1],++totalTyped);
      typingTimer=setTimeout(tick,row.classList.contains('terminal-command')?26:16);
    }else{character=0;textIndex++;typingTimer=setTimeout(tick,8);}
  }
  typingTimer=setTimeout(tick,150);
}
function openCommand(prefix=''){closeAgentPicker(); $('.command').hidden=false;$('.command-input').value=prefix;$('.command-input').focus();historyIndex=commandHistory.length; }
function autonomyPanel(policy){
  const details=node('details','calculation autonomy-panel'),summary=node('summary','','Run an offline autonomy workflow');details.append(summary);
  details.append(node('p','terminal-notice','A new test agent receives your goal, selects permitted tools for three deterministic cycles, then pauses. No GPT calls, external actions or real spending.'));
  details.append(node('p','prompt','Offline progress estimates are execution heuristics, not proof that the goal was completed or a measure of intelligence.'));
  details.append(node('p','prompt',`Tools: ${policy.tools.join(', ')}. Simulated seed: ${policy.simulated_seed_microusd/1000000} USD; maximum simulated expenses: ${policy.simulated_expense_limit_microusd/1000000} USD.`));
  const form=node('form','autonomy-form'),label=node('label','','Goal / operator-only'),goal=node('textarea');goal.id='autonomy-goal';goal.name='goal';goal.rows=3;goal.maxLength=500;goal.minLength=3;goal.required=true;label.htmlFor=goal.id;
  const consentLabel=node('label','autonomy-consent'),consent=node('input');consent.type='checkbox';consent.required=true;consent.name='authorize_offline';consentLabel.append(consent,document.createTextNode('I authorize a new test identity, three offline cycles and up to 0.0033 simulated USD of expenses. No real funds.'));
  const run=node('button','','Run authorized offline workflow');run.type='submit';form.append(label,goal,consentLabel,run);
  const status=node('p','terminal-notice');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
  const inspect=node('button','','Inspect saved run'),reset=node('button','','Prepare another workflow');inspect.type=reset.type='button';
  function sync(){
    form.hidden=!!autonomyAttempt;inspect.hidden=!autonomyAttempt;reset.hidden=!autonomyAttempt||autonomyAttempt.status==='pending';inspect.disabled=autonomyAttempt?.status==='pending';
    status.textContent=!autonomyAttempt?'Authorization is required for each run.':autonomyAttempt.status==='pending'?`Running offline workflow ${autonomyAttempt.id}…`:autonomyAttempt.status==='completed'?`Saved ${autonomyAttempt.cycles} completed offline cycles. Agent paused. Request ${autonomyAttempt.id}.`:`Outcome not confirmed here. Inspect saved run ${autonomyAttempt.id} before starting another workflow. No automatic retry.`;
  }
  form.addEventListener('submit',async event=>{
    event.preventDefault();event.stopPropagation();if(autonomyAttempt||!form.reportValidity()||!consent.checked)return;
    const attempt=autonomyAttempt={id:crypto.randomUUID(),status:'pending'};try{sessionStorage.setItem('supa.offline-request',attempt.id);}catch{/* Durable server identity still prevents duplicate execution. */}sync();
    try{
      const response=await fetch('/api/autonomy',{method:'POST',credentials:'same-origin',cache:'no-store',headers:{'Content-Type':'application/json','X-Supa-Action':'offline-autonomy'},body:JSON.stringify({request_id:attempt.id,goal:goal.value.trim(),authorize_offline:true}),signal:AbortSignal.timeout(35000)});
      const result=await response.json();
      if(!response.ok||result.agent_id!==attempt.id||result.mode!=='offline'||result.lifecycle!=='paused'||result.paid_requests!==0||!Array.isArray(result.cycles)||result.cycles.length!==3)throw Error();
      attempt.status='completed';attempt.cycles=result.cycles.length;
    }catch{attempt.status='unknown';}
    sync();$('.announcement').textContent=attempt.status==='completed'?'Offline workflow saved; agent paused.':'Workflow outcome uncertain. Inspect saved records; do not retry automatically.';
  });
  inspect.addEventListener('click',async()=>{
    const id=autonomyAttempt?.id;if(!id)return;await reload();
    if(!state.agents.data?.some(agent=>agent.id===id)){status.textContent='No saved identity was returned for this request. Check backend activity before preparing another workflow.';return;}
    agentId=id;$('#agent-select').value=id;syncAgentPicker();wheel.select(2);void select(2);
  });
  reset.addEventListener('click',()=>{if(autonomyAttempt?.status==='pending')return;autonomyAttempt=null;goal.value='';consent.checked=false;try{sessionStorage.removeItem('supa.offline-request');}catch{}sync();goal.focus();});
  details.append(form,status,inspect,reset);sync();return details;
}
function closeCommand(focus=true){$('.command').hidden=true;if(focus)returnFocus();}
function returnFocus(){if(document.body.dataset.view==='intro'){$('.enter').focus({preventScroll:true});return;}if(matchMedia('(max-width:700px)').matches){setDrawer(false,false);$('.tabline').focus({preventScroll:true});}else $('.wheel').focus({preventScroll:true});}
$('.command-toggle').addEventListener('click',()=>openCommand('supa '));
$('.command-close').addEventListener('click',()=>closeCommand());
$('.command-input').addEventListener('keydown',event=>{if(['ArrowUp','ArrowDown'].includes(event.key)){event.preventDefault();historyIndex=Math.max(0,Math.min(commandHistory.length,historyIndex+(event.key==='ArrowUp'?-1:1)));event.target.value=commandHistory[historyIndex] ?? '';}});
$('.command').addEventListener('submit',event=>{
  event.preventDefault();const value=$('.command-input').value.trim().replace(/^:/,'');if(value){commandHistory.push(value);commandHistory=commandHistory.slice(-50);}closeCommand();
  if(value==='theme'){changeTheme();return;}
  if(['q','quit'].includes(value)){history.pushState({},'','/');setView(false);$('.enter').focus();return;}
  const command=value.replace(/^supa\s+/,'');
  if(command==='help'){line('UI commands only: supa status, supa agents, supa inspect <name or UUID>, supa intelligence, supa memory, supa economy, supa social, supa capabilities. ↑↓ choose views, j/k scroll, : opens command, Escape returns navigation. No shell execution.','terminal-notice');$('.announcement').textContent='Command help added to terminal.';return;}
  if(command.startsWith('inspect ')){
    const query=command.slice(8).toLowerCase();const agent=state.agents.data?.find(a=>a.id===query||a.name.toLowerCase()===query);
    if(!agent){$('.announcement').textContent='No matching saved agent. Use supa agents.';return;}
    agentId=agent.id;$('#agent-select').value=agentId;syncAgentPicker();wheel.select(2);void select(2);return;
  }
  const id=({status:'home',capabilities:'about'})[command]??command,index=navigation.findIndex(item=>item.id===id);
  if(index>=0){if(document.body.dataset.view==='intro'){history.pushState({},'','/site/');setView(true);}wheel.select(index);void select(index);}
  else{$('.announcement').textContent='Unknown command. Try supa help.';line('Unknown command. Try supa help.','terminal-notice');}
});
$('.worm-switch').addEventListener('click',()=>{worm=!worm;$('.worm-switch').setAttribute('aria-pressed',String(worm));$('.worm-switch').textContent=worm?'awake':'asleep';scenes?.setWorm(worm);});
window.addEventListener('keydown',async event=>{
  if(event.defaultPrevented||event.ctrlKey||event.metaKey||event.altKey)return;
  if(event.key==='Escape'){closeCommand(false);returnFocus();return;}
  if(event.target instanceof HTMLInputElement||event.target instanceof HTMLSelectElement||event.target instanceof HTMLTextAreaElement)return;
  if(event.key==='Tab'&&$('.projects').classList.contains('drawer-open')){
    const active=$('.wheel-item[aria-selected=true]');if(event.shiftKey&&document.activeElement===$('.tabline')){event.preventDefault();active?.focus();}else if(!event.shiftKey&&document.activeElement===active){event.preventDefault();$('.tabline').focus();}return;
  }
  if(!await ready)return;scenes.onInput(event.key);
  if(event.key===':'){event.preventDefault();openCommand();return;}
  if(document.body.dataset.view==='intro'){if(event.key==='Enter'&&!event.target.closest('button,a'))await enter(event);return;}
  if(event.target instanceof Element&&event.target.closest('.buffer')){const scroll={j:24,ArrowDown:24,k:-24,ArrowUp:-24,d:$('.buffer').clientHeight/2,u:-$('.buffer').clientHeight/2};if(event.key in scroll){event.preventDefault();$('.buffer').scrollBy({top:scroll[event.key]});return;}if(event.key==='h'){event.preventDefault();returnFocus();return;}}
  const steps={j:1,ArrowDown:1,k:-1,ArrowUp:-1};
  if(event.key in steps){event.preventDefault();wheel.step(steps[event.key]);}
  else if(event.key==='g')$('.buffer').scrollTo({top:0});
  else if(event.key==='G')$('.buffer').scrollTo({top:$('.buffer').scrollHeight});
});
window.addEventListener('pagehide',event=>{if(!event.persisted){clearInterval(liveTimer);cancelTyping();wheel?.destroy();scenes?.destroy();}});
