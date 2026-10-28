import { createWheel } from './wheel.js';
import { createScenes } from './scene.js';

const $ = selector => document.querySelector(selector);
const root = document.documentElement;
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
reduced.addEventListener('change',event=>{root.dataset.motion=event.matches?'still':'running';});
root.dataset.motion = reduced.matches ? 'still' : 'running';
const format = value => Number(value).toLocaleString('en-US').replaceAll(',', "'");
const node = (tag, className, text) => { const el = document.createElement(tag); if (className) el.className = className; if (text !== undefined) el.textContent = String(text); return el; };
const palettes = [['#0c0c0c','#d8d4b4','#949078'],['#0b0b0b','#ebe6db','#9b968a'],['#11120f','#d9d7c3','#9b9d85']];
let theme = 0, wheel, scenes, data, itemIndex = 0, shot = 0, typing = 0, leaving = false, worm = false;
let items = [], selectedProject = null, typingTimer = 0;
try { const saved=Number(localStorage.getItem('reference.dark-theme')); if(Number.isInteger(saved)&&saved>=0&&saved<palettes.length)theme=saved; } catch { /* Storage may be disabled. */ }
function applyTheme() { ['--bg','--ink','--dim'].forEach((name,index)=>root.style.setProperty(name,palettes[theme][index])); }
applyTheme();
const ready = boot();

async function boot() {
  try {
    const response = await fetch('/reference-data.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('snapshot_unavailable');
    data = await response.json();
    if (!Array.isArray(data.projects) || !Array.isArray(data.days)) throw new Error('snapshot_invalid');
    const byName = name => data.projects.find(p => p.repo === name);
    items = [{label:'whoami', kind:'identity', thumb:'https://hancore.dev/favicon.svg'}, ...['Shibumi-Shell','quickshell-dots','waybar-themes','OmaQ'].map(name => ({label:name, kind:'project', project:byName(name), thumb:byName(name)?.images[0]})), ...['omarchy/','more/','stack/','themes/'].map(label => ({label,kind:'directory',thumb:label==='omarchy/'?'https://hancore.dev/site/brand/omarchy-wordmark.svg':undefined}))];
    scenes = createScenes({intro:$('.intro-canvas'),blueprint:$('.blueprint'),entity:$('.entity'),transition:$('.transition'),spark:$('.spark'),reduced,data});
    document.fonts.ready.then(()=>scenes?.redraw());
    fetch('/app.js').then(r=>r.ok?r.text():'').then(text=>{if(text)scenes?.setCode(text);}).catch(()=>{});
    wheel = createWheel({host:$('.wheel'),canvas:$('.wheel-ink'),items,reducedMotion:reduced,onSelect:index=>select(index)});
    $('[data-stat=repos]').textContent = format(data.repositoryCount);
    $('[data-stat=stars]').textContent = format(data.stars);
    $('[data-stat=followers]').textContent = format(data.followers);
    $('.snapshot-date').textContent = data.fetchedAt.slice(0,10);
    $('.bar').title = `Public Hancore GitHub snapshot, ${data.fetchedAt}. Not local or Supa statistics.`;
    const initialHash=location.hash;
    setView(location.pathname.startsWith('/site'));
    if(document.body.dataset.view==='projects')openHash(initialHash);
    root.dataset.ready = 'true';
    return true;
  } catch {
    const message = node('p','load-error','The saved reference snapshot could not be loaded. Reload this page to try again.');
    $('.intro').append(message); $('.announcement').textContent = message.textContent;
    root.dataset.ready = 'error';
    return false;
  }
}

function setView(projects) {
  typing++;clearTimeout(typingTimer);$('.buffer').setAttribute('aria-busy','false');
  document.body.dataset.view = projects ? 'projects' : 'intro';
  $('.intro').hidden = projects; $('.projects').hidden = !projects;
  closeCommand(false); setDrawer(false, false);
  scenes?.setMode(projects ? 'projects' : 'intro');
  if (projects) { select(itemIndex); requestAnimationFrame(() => { window.dispatchEvent(new Event('resize')); $('.buffer').focus({preventScroll:true}); }); }
}
async function enter(event) {
  event?.preventDefault(); if (leaving || !await ready) return;
  leaving = true; await scenes.warp();
  history.pushState({view:'projects'},'', '/site/'); setView(true); leaving = false;
}
$('.enter').addEventListener('click', enter);
$('.brand').addEventListener('click', async event => {
  event.preventDefault(); if (!await ready) return;
  history.pushState({view:'intro'},'', '/'); setView(false); $('.enter').focus({preventScroll:true});
});
window.addEventListener('popstate', () => { typing++; leaving = false; const hash=location.hash; setView(location.pathname.startsWith('/site'));if(document.body.dataset.view==='projects')openHash(hash); });
window.addEventListener('hashchange',()=>{if(root.dataset.ready==='true'&&document.body.dataset.view==='projects')openHash(location.hash);});
function openHash(hash) {
  let id='about';try{id=decodeURIComponent(hash.replace(/^#/,''))||'about';}catch{/* Invalid fragments return to the identity. */}
  const index=items.findIndex(item=>(item.kind==='identity'?'about':item.label.replace(/\/$/,''))===id);
  if(index>=0){wheel.select(index);select(index);return;}
  const project=data.projects.find(p=>p.repo===id);if(project){const parent=items.findIndex(i=>i.label===(id==='omarchy-plugin-marketplace'?'omarchy/':'more/'));wheel.select(parent);renderProject(project);}else{wheel.select(0);select(0);}
}

function changeTheme() {
  theme = (theme + 1) % palettes.length;
  applyTheme();
  try { localStorage.setItem('reference.dark-theme',String(theme)); } catch { /* Optional local preference. */ }
  scenes?.redraw(); window.dispatchEvent(new Event('resize'));
  $('.announcement').textContent = `Dark theme ${theme + 1} of ${palettes.length}`;
}
$('.theme-key').addEventListener('click', changeTheme);
function setDrawer(open, focus = true) {
  $('.projects').classList.toggle('drawer-open',open);
  $('.tabline').setAttribute('aria-expanded',String(open));
  if (focus) (open ? $('.wheel') : $('.tabline')).focus({preventScroll:true});
  requestAnimationFrame(()=>window.dispatchEvent(new Event('resize')));
}
$('.tabline').addEventListener('click',()=>setDrawer($('.tabline').getAttribute('aria-expanded')!=='true'));

function line(text, className = 'terminal-line') { const el = node('p',className,text); $('.buffer').append(el); return el; }
function prompt(command) { const p = line('','prompt'); p.append(node('span','','hancore@dev '),node('em','','~')); line(`› ${command}`,'terminal-command'); }
function linkLine(label,url) { const p = line(''); const a = node('a','',label); a.href = url; a.target = '_blank'; a.rel = 'noopener noreferrer'; p.append(a); }
function select(index) {
  itemIndex = index; shot = 0; typing++; selectedProject = null;
  const item = items[index]; if (!item) return;
  if(document.body.dataset.view==='projects')history.replaceState(null,'',`#${item.kind==='identity'?'about':item.label.replace(/\/$/,'')}`);
  $('.tab-file').textContent = item.label; $('.winbar').textContent = item.kind === 'identity' ? 'fish — ~ · HANCORE-linux' : `${item.label} — public project snapshot`;
  const buffer = $('.buffer'); buffer.replaceChildren();
  $('.pane').classList.remove('is-changing'); void buffer.offsetWidth; $('.pane').classList.add('is-changing');
  scenes?.setSelection(index);
  if (item.kind === 'identity') {
    $('.blueprint').hidden = false;
    prompt('whoami');
    line('Mechanical engineering by day, with a love for design and development. Exploring technology, always curious, and building tools for the Omarchy community.');
    line(''); prompt('fetch HANCORE-linux');
    line(`followers ${format(data.followers)}\nstars     ${format(data.stars)}\nrepos     ${format(data.repositoryCount)}\ncontribs  ${format(data.contributions)}   ${data.fetchedAt.slice(0,10)}`);
    line(''); prompt('ls links/'); linkLine('github','https://github.com/HANCORE-linux');
    line(''); prompt(''); line('','terminal-line').append(node('span','terminal-prompt'));
  } else if (item.kind === 'project' && item.project) {
    renderProject(item.project, false);
  } else {
    $('.blueprint').hidden = item.label !== 'stack/';
    prompt(`ls ${item.label}`); line('');
    if (item.label === 'more/' || item.label === 'omarchy/') {
      const list = item.label === 'more/' ? data.projects.filter(p=>['omarchy-bongocat','cliamp-plugin-nightrider','GitHub-Traffic-Board'].includes(p.repo)) : data.projects.filter(p=>p.repo==='omarchy-plugin-marketplace');
      for (const p of list) { const button=node('button','terminal-entry',`${p.repo} / ${p.language || 'project'} · ${format(p.stars)} stars`);button.addEventListener('click',()=>renderProject(p)); buffer.append(button); }
    } else if (item.label === 'stack/') {
      line([...new Set(data.projects.map(p=>p.language).filter(Boolean))].join(' · '));
      line('Languages represented in the captured project metadata.');
    } else {
      line('Dark color studies. Select a swatch or run :theme.');
      const swatches = node('div','swatches');
      palettes.forEach((palette,index)=>{const button=node('button','',`[ ${index+1} ]`);button.style.color=palette[1];button.setAttribute('aria-label',`Dark theme ${index+1}`);button.addEventListener('click',()=>{theme=(index+palettes.length-1)%palettes.length;changeTheme();});swatches.append(button);});
      buffer.append(swatches);
    }
    line(''); prompt('');
  }
  buffer.scrollTop = 0;
  if (matchMedia('(max-width:700px)').matches) setDrawer(false,false);
  $('.announcement').textContent = `Selected ${item.label}`;
  $('.key-feedback').textContent = `↑↓ project · ↔ shot · ↩ enter · : cmd`;
  typeTerminal();
}
function renderProject(project, animate = true) {
  typing++; clearTimeout(typingTimer);
  selectedProject = project; shot = 0; history.replaceState(null,'',`#${project.repo}`); $('.buffer').replaceChildren(); $('.blueprint').hidden = true;
  $('.tab-file').textContent = project.repo;
  prompt(`open ${project.repo}`); line('');
  line(`${project.language || 'Open-source'} project · ${format(project.stars)} stars · ${project.forks} forks`);
  line(`snapshot ${data.fetchedAt.slice(0,10)} · last pushed ${project.pushed}`);
  renderImage(project); linkLine('view repository ↗',project.url);
  line(''); prompt(''); line('← → browse images · Enter expand · Escape return to wheel','prompt');
  $('.buffer').scrollTop = 0;
  if(animate)typeTerminal();
}
// Recreate the terminal's progressive reveal using this study's own text.
function typeTerminal() {
  clearTimeout(typingTimer);
  const buffer=$('.buffer'), token=++typing;
  const rows=[...buffer.children];
  const ghost=document.querySelectorAll('.entity-state b')[1];
  if(reduced.matches){buffer.setAttribute('aria-busy','false');buffer.setAttribute('aria-live','polite');if(ghost)ghost.textContent='idle';return;}
  const originals=[];
  rows.forEach(row=>{
    row.hidden=true;
    const walker=document.createTreeWalker(row,NodeFilter.SHOW_TEXT);
    let text;
    while((text=walker.nextNode())){originals.push({text,value:text.textContent,row});text.textContent='';}
  });
  buffer.setAttribute('aria-busy','true');buffer.setAttribute('aria-live','off');
  if(ghost)ghost.textContent='typing';
  let rowIndex=0, textIndex=0, character=0;
  function finish(){
    originals.forEach(({text,value})=>{text.textContent=value;});rows.forEach(row=>{row.hidden=false;});
    buffer.setAttribute('aria-busy','false');buffer.setAttribute('aria-live','polite');if(ghost)ghost.textContent='idle';
  }
  function tick(){
    if(token!==typing)return;
    if(reduced.matches){finish();return;}
    const row=rows[rowIndex];if(!row){finish();return;}
    row.hidden=false;
    const record=originals[textIndex];
    if(!record||record.row!==row){rowIndex++;typingTimer=setTimeout(tick,150);return;}
    if(character<record.value.length){
      character++;record.text.textContent=record.value.slice(0,character);
      typingTimer=setTimeout(tick,row.classList.contains('terminal-command')?26:12);
    }else{character=0;textIndex++;typingTimer=setTimeout(tick,8);}
  }
  typingTimer=setTimeout(tick,200);
}
function renderImage(project) {
  $('.terminal-artwork')?.remove();
  if (!project.images.length) { line('No preview image is listed in the snapshot.'); return; }
  const image = node('img','terminal-artwork'); image.src = project.images[shot % project.images.length]; image.alt = `${project.repo} reference artwork, image ${shot+1} of ${project.images.length}`; image.referrerPolicy = 'no-referrer';
  image.addEventListener('click',()=>image.classList.toggle('zoomed'));
  image.addEventListener('error',()=>{ image.replaceWith(node('p','prompt','Remote reference artwork is unavailable. The repository link remains available.')); },{once:true});
  const lines = $('.buffer'); lines.append(image);
}
function nextImage(delta) {
  const project = selectedProject || items[itemIndex]?.project;
  if (!project?.images.length) return;
  shot = (shot + delta + project.images.length) % project.images.length; renderImage(project);
  $('.key-feedback').textContent = `image ${shot+1} / ${project.images.length}`;
}
function openCommand(prefix = '') { $('.command').hidden = false; $('.command-input').value = prefix; $('.command-input').focus(); }
function closeCommand(focus = true) { $('.command').hidden = true; if (focus) returnFocus(); }
function returnFocus() { if(document.body.dataset.view==='intro'){$('.enter').focus({preventScroll:true});return;} if (matchMedia('(max-width:700px)').matches) { setDrawer(false,false); $('.tabline').focus({preventScroll:true}); } else $('.wheel').focus({preventScroll:true}); }
$('.command').addEventListener('submit',event=>{
  event.preventDefault(); const value = $('.command-input').value.trim().replace(/^:/,''); closeCommand();
  if (value === 'theme' || value.startsWith('colorscheme')) changeTheme();
  else if (['q','quit'].includes(value)) { history.pushState({},'', '/'); setView(false); $('.enter').focus(); }
  else if (value === 'help') { $('.announcement').textContent = 'j/k or arrows select; left/right browse images; Enter expands; theme changes palette; q returns to intro.'; if (document.body.dataset.view === 'projects') line('j/k ↑↓ projects · ←→ images · Enter expand · :theme · :q','prompt'); }
  else if (value.startsWith('e ')) { const match=items.findIndex(i=>i.label.toLowerCase().replace(/\/$/,'')===value.slice(2).toLowerCase().replace(/\/$/,'')); if(match>=0)wheel.select(match);else { const project=data.projects.find(p=>p.repo.toLowerCase()===value.slice(2).toLowerCase());if(project)openHash('#'+project.repo);else $('.announcement').textContent='No matching project in the saved snapshot.'; } }
  else $('.announcement').textContent = 'Unknown command. Available commands: theme, e project, help, q.';
});
$('.worm-switch').addEventListener('click',()=>{worm=!worm;$('.worm-switch').setAttribute('aria-pressed',String(worm));$('.worm-switch').textContent=worm?'awake':'asleep';scenes?.setWorm(worm);});

window.addEventListener('keydown',async event=>{
  if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key === 'Escape') { closeCommand(false); returnFocus(); return; }
  if (event.target instanceof HTMLInputElement) return;
  if (!await ready) return;
  scenes.onInput(event.key); $('.key-feedback').textContent = `key / ${event.key === ' ' ? 'space' : event.key}`;
  if (event.key === ':') { event.preventDefault(); openCommand(); return; }
  if (document.body.dataset.view === 'intro') { if (event.key === 'Enter' && !event.target.closest('button,a')) await enter(event); return; }
  const inBuffer=event.target instanceof Element&&event.target.closest('.buffer');
  if(inBuffer){const scroll={j:13,ArrowDown:13,k:-13,ArrowUp:-13,d:$('.buffer').clientHeight/2,u:-$('.buffer').clientHeight/2};if(event.key in scroll){event.preventDefault();$('.buffer').scrollBy({top:scroll[event.key]});return;}if(event.key==='h'){event.preventDefault();returnFocus();return;}}
  const steps = {j:1,ArrowDown:1,k:-1,ArrowUp:-1};
  if (event.key in steps) { event.preventDefault(); wheel.step(steps[event.key]); }
  else if (event.key === 'ArrowRight') { event.preventDefault(); nextImage(1); }
  else if (event.key === 'ArrowLeft') { event.preventDefault(); nextImage(-1); }
  else if (event.key === 'Enter' && !event.target.closest('button,a')) { event.preventDefault(); $('.terminal-artwork')?.classList.toggle('zoomed'); }
  else if (event.key === 'g') $('.buffer').scrollTo({top:0});
  else if (event.key === 'G') $('.buffer').scrollTo({top:$('.buffer').scrollHeight});
});
window.addEventListener('pagehide',event=>{if(!event.persisted){typing++;clearTimeout(typingTimer);wheel?.destroy();scenes?.destroy();}});
