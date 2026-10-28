// Independently authored procedural scenes, informed by observed browser behavior.
// No reference scripts, filmed portrait or tracking code are included.
export function createScenes({ intro, blueprint, entity, transition, spark, reduced, data }) {
  let mode = 'intro', frame = 0, previous = 0, warpStart = 0, warpResolve, input = '', selected = 0, worm = false;
  const pointer = { x: .5, y: .5, visible: false };
  const tether = document.createElement('canvas');
  tether.className='pointer-tether';tether.setAttribute('aria-hidden','true');
  tether.style.cssText='position:fixed;inset:0;width:100%;height:100%;pointer-events:none;z-index:8';
  document.body.append(tether);
  let introProgress = 0, introTime = null, handBeat = -1, handKey = "";
  let diagramTime=0,diagramPrevious=null,diagramVisible=false,diagramPaused=false;
  const diagramControl=document.querySelector('.diagram-motion');
  const contexts = new Map();
  const portalBrand = document.querySelector('.portal-brand');
  let code = [
    '// workspace / a small study in motion', '',
    'import { clock, cursor } from "./system.js";',
    'const view = new Scene({ width, height });',
    'const state = { ready: false, frame: 0 };', '',
    'function observe(position) {',
    '  const direction = position.subtract(center);',
    '  return direction.normalize();',
    '}', '',
    'export function render(time) {',
    '  const delta = clock.advance(time);',
    '  const target = observe(cursor.position);',
    '  state.frame += 1;',
    '  view.clear(palette.background);',
    '  view.lines.forEach((line, index) => {',
    '    const phase = index / view.lines.length;',
    '    line.follow(target, delta * phase);',
    '    line.draw(view.context);',
    '  });',
    '  return state;',
    '}', '',
    'function onKey(event) {',
    '  const key = event.key;',
    '  if (key === "Enter") openProjects();',
    '  if (key === "Escape") returnToWheel();',
    '  cursor.remember(key);',
    '}', '',
    'window.addEventListener("keydown", onKey);',
    '// leave space for the next idea',
  ];
  const ink = () => getComputedStyle(document.documentElement).getPropertyValue('--ink').trim();
  const bg = () => getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
  function fit(canvas) {
    const box = canvas.getBoundingClientRect(), dpr = Math.min(devicePixelRatio || 1, 2);
    if (!box.width || !box.height) return null;
    if (canvas.width !== Math.round(box.width * dpr) || canvas.height !== Math.round(box.height * dpr)) {
      canvas.width = Math.round(box.width * dpr); canvas.height = Math.round(box.height * dpr);
    }
    let ctx = contexts.get(canvas);
    if (!ctx) { ctx = canvas.getContext('2d'); contexts.set(canvas, ctx); }
    if (!ctx) return null;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0); ctx.clearRect(0, 0, box.width, box.height);
    ctx.fillStyle = ink(); ctx.strokeStyle = ink();
    return { ctx, w: box.width, h: box.height };
  }
  function ascii(ctx, x, y, width, height, time, small = false) {
    // Code texture moves inside the canonical mask; the robot never deforms.
    const silhouette = new Path2D('M7 2h10v2h4v4h2v10h-4v4H5v-4H1V8h2V4h4zM6 9h4v5H6zm8 0h4v5h-4zM8 17h8v2H8z');
    const size=Math.min(width,height),scale=size/24;
    ctx.save();ctx.translate(x+(width-size)/2,y+(height-size)/2);ctx.scale(scale,scale);
    ctx.clip(silhouette,'evenodd');ctx.font='.72px Plex,monospace';
    ctx.textBaseline='top';ctx.fillStyle=ink();
    const source=code.join(' ').replace(/\s+/g,' '),offset=reduced.matches?0:Math.floor(time/1800);
    for(let row=0;row<30;row++)for(let column=0;column<48;column++){
      ctx.globalAlpha=.35+.25*((row+column)%4)/3;
      ctx.fillText(source[(row*48+column+offset)%source.length],column*.5,row*.8);
    }
    ctx.restore();
  }
  // Decorative hands share the editor's typing clock; no camera or key capture.
  function drawHands(ctx, w, h, typing, character) {
    const width=Math.min(700,w-40,h*.8),pitch=width/15,rowPitch=pitch*.68,keyHeight=pitch*.54;
    const left=(w-width)/2,top=h*.755,height=rowPitch*4.65;
    const rows=[['esc','`','1','2','3','4','5','6','7','8','9','0','-','=','back'],['tab','q','w','e','r','t','y','u','i','o','p','[',']','\\'],['caps','a','s','d','f','g','h','j','k','l',';',"'",'enter'],['shift','z','x','c','v','b','n','m',',','.','/','shift']];
    const keys=[],byKey=new Map();
    rows.forEach((row,r)=>{
      const offset=[0,.25,.5,.75][r]*pitch;
      row.forEach((label,c)=>{
        const key={label,x:left+offset+c*pitch,y:top+r*rowPitch,w:pitch-4,h:keyHeight};
        key.cx=key.x+key.w/2;key.cy=key.y+key.h/2;keys.push(key);byKey.set(label,key);
      });
    });
    const space={label:' ',x:w/2-width*.25,y:top+4*rowPitch,w:width*.5,h:keyHeight};
    space.cx=w/2;space.cy=space.y+space.h/2;keys.push(space);byKey.set(' ',space);
    const aliases={'{':'[','}':']','(': '9',')':'0',':':';','"':"'",'<':',','>':'.','|':'\\','_':'-','+':'=','!':'1'};
    const beat=typing?Math.floor(introProgress/5):-1;
    if(beat!==handBeat){handBeat=beat;handKey=aliases[character]||character.toLowerCase();}
    const active=typing?byKey.get(handKey):null,pulse=typing?Math.sin((introProgress/5%1)*Math.PI):0;
    const leftFingers=[' ','45rtfgvb','3edc','2wsx','1qaz'];
    const rightFingers=[' ','67yuhjnm','8ik,','9ol.','0p;/-=[]\\\''];
    let activeHand=handBeat%2,activeFinger=0;
    if(active&&handKey!==' '){
      const li=leftFingers.findIndex(group=>group.includes(handKey));
      const ri=rightFingers.findIndex(group=>group.includes(handKey));
      if(li>=0){activeHand=0;activeFinger=li;}else if(ri>=0){activeHand=1;activeFinger=ri;}
    }
    ctx.save();ctx.font=`${Math.max(8,Math.min(13,pitch*.3))}px Plex,monospace`;
    ctx.textAlign='center';ctx.textBaseline='middle';ctx.lineWidth=.6;
    for(const key of keys){
      const pressed=key===active;
      ctx.fillStyle=pressed?'#71957d':ink();ctx.strokeStyle=ink();
      if(pressed){ctx.globalAlpha=.12+pulse*.2;ctx.fillRect(key.x,key.y,key.w,key.h);}
      ctx.globalAlpha=pressed?.65:.16;ctx.strokeRect(key.x,key.y,key.w,key.h);
      ctx.globalAlpha=pressed?.95:.42;ctx.fillText(key.label===' '?'space':key.label,key.cx,key.cy+(pressed?pulse*2:0));
    }
    const connections=[[0,1,2,3,4],[0,5,6,7,8],[0,9,10,11,12],[0,13,14,15,16],[0,17,18,19,20],[5,9,13,17]];
    for(const [hand,sign,home] of [[0,-1,[' ','f','d','s','a']],[1,1,[' ','j','k','l',';']]]){
      const wristX=w/2+sign*width*.145,wristY=top+height+pitch*.36,points=[[wristX,wristY]];
      for(let finger=0;finger<5;finger++){
        const rest=byKey.get(home[finger]),press=active&&hand===activeHand&&finger===activeFinger;
        const tipX=press?active.cx:rest.cx+(finger===0?sign*pitch*.65:0);
        const tipY=(press?active.cy:rest.cy)-pitch*.18+(press?pulse*pitch*.23:0);
        const baseX=wristX+(rest.cx-wristX)*.65,baseY=top+height*.85;
        for(let joint=0;joint<4;joint++){
          const t=joint/3;
          points.push([baseX+(tipX-baseX)*t,baseY+(tipY-baseY)*t-(press?Math.sin(t*Math.PI)*pulse*pitch*.12:0)]);
        }
      }
      ctx.strokeStyle=ink();ctx.fillStyle=ink();ctx.globalAlpha=.6;ctx.lineWidth=.8;
      for(const path of connections){ctx.beginPath();path.forEach((id,i)=>{if(i)ctx.lineTo(...points[id]);else ctx.moveTo(...points[id]);});ctx.stroke();}
      ctx.globalAlpha=.85;for(const [px,py] of points)ctx.fillRect(px-1,py-1,2,2);
    }
    ctx.restore();
    intro.dataset.handState=typing?'typing':'idle';
    intro.dataset.keyboardBounds=[left,top,width,height+pitch*.42].map(n=>n.toFixed(2)).join(',');
  }
  function drawIntro(time) {
    const view = fit(intro); if (!view) return;
    const { ctx, w, h } = view, narrow=w<700;
    const font=narrow?11:Math.min(16,Math.max(12,Math.round(w/110))),line=Math.round(font*1.17),cell=font*.6;
    const left=narrow?20:w*.18,width=narrow?w-40:w*.66,top=h*.125,first=top+font*3,bottom=h*.7;
    const rows=Math.max(8,Math.floor((bottom-first)/line));
    ctx.font=`${font}px Plex,monospace`;
    if(reduced.matches) introProgress=Infinity;
    else if(introTime!==null) introProgress+=Math.max(0,Math.min(100,time-introTime))/30;
    introTime=time;
    let remaining=introProgress,revealedRows=0,typedCharacters=0,activeRow=0,activeColumn=0,typedKey=" ";
    ctx.save();ctx.beginPath();ctx.rect(left,top,width,bottom-top);ctx.clip();
    ctx.globalAlpha=.25;ctx.fillText('// — reference-web/app.js —',left+cell*3,top+font);
    for(let row=0;row<rows;row++){
      const limit=Math.max(1,Math.floor(width/cell)-5),full=code[row%code.length].slice(0,limit);
      const count=Math.min(full.length,Math.max(0,Math.floor(remaining))),started=remaining>=0,complete=remaining>=full.length+8;
      remaining-=full.length+8;
      if(complete)revealedRows++;
      typedCharacters+=count;
      if(!started)continue;
      activeRow=row;activeColumn=count;if(count>0)typedKey=full[count-1];
      const baseline=first+row*line;
      if(!complete){ctx.globalAlpha=.7;ctx.fillStyle='#161615';ctx.fillRect(left,baseline-font,width,line);}
      ctx.fillStyle=ink();ctx.globalAlpha=.28;ctx.fillText(String(row+1).padStart(3),left,baseline);
      const text=full.slice(0,count),textX=left+cell*4;
      // Authored source tokens receive the reference's muted terminal palette.
      const tokens=text.split(/(\b(?:import|from|const|let|function|return|export|if|async|await|try|catch)\b|['"`][^'"`]*['"`]|\b\d+\b)/g);
      let tx=textX;
      for(const token of tokens){ctx.globalAlpha=.42;ctx.fillStyle=/^(import|from|const|let|function|return|export|if|async|await|try|catch)$/.test(token)?'#71957d':/^\d+$/.test(token)?'#cd796e':ink();ctx.fillText(token,tx,baseline);tx+=ctx.measureText(token).width;}
      if(!complete&&!reduced.matches&&Math.floor(time/550)%2===0){ctx.fillStyle=ink();ctx.globalAlpha=.7;ctx.fillRect(textX+count*cell,baseline-font,cell,line);}
    }
    ctx.restore();
    intro.dataset.revealedRows=String(revealedRows);intro.dataset.typedCharacters=String(typedCharacters);intro.dataset.introState=revealedRows===rows?'complete':'typing';
    const status=document.querySelector('.intro-modes'),label=document.querySelector('.mode-label'),position=document.querySelector('.intro-position');
    const state=revealedRows===rows?'NORMAL':'INSERT',cursor=`${activeRow+1}:${activeColumn+1}`;
    if(status)status.dataset.mode=state;
    if(label&&label.textContent!==state)label.textContent=state;
    if(position&&position.textContent!==cursor)position.textContent=cursor;
    drawHands(ctx,w,h,!reduced.matches&&revealedRows<rows,typedKey);
    ctx.globalAlpha=1;
  }
  function drawBlueprint() {
    const view = fit(blueprint); if (!view) return;
    const { ctx, w, h } = view, compact = w < 400, split = compact ? w : w * .62, rootX = w * (compact ? .3 : .15), baseY = h * (compact ? .88 : .92);
    const phase=reduced.matches?0:diagramTime/1000;
    blueprint.dataset.motionPhase=String(phase);
    ctx.font = '11px Plex,monospace'; ctx.globalAlpha = .7;
    ctx.fillText('SUPA / architecture · decorative branches', 9, 16);
    if(!compact) ctx.fillText('section A—A', split + 10, 24);
    ctx.lineWidth = .5;
    for (let r = 0; r < 45; r++) { const y = 32 + r * (h - 45) / 45; ctx.globalAlpha = .1; ctx.beginPath(); ctx.moveTo(15, y); ctx.lineTo(split - 10, y); ctx.stroke(); }
    ctx.globalAlpha = .7;
    for (let n = 0; n < 35; n++) {
      const fork = baseY - (n / 35) * h * .45;
      const side = n % 6 === 0 ? -.3 : .7;
      const sway=Math.sin(phase*.55+n*.19)*3;
      const tipX = Math.min(split - 13, rootX + (n % 6 === 0 ? 20 : split * .58)) - Math.sin(n * .9) * 8+sway;
      const tipY = 50 + n * (h - 95) / 41+sway*.4;
      const c1x=rootX-n*side,c2x=rootX+(tipX-rootX)*.45,c2y=tipY+n;
      ctx.globalAlpha = .25 + n / 70; ctx.beginPath(); ctx.moveTo(rootX, baseY);
      ctx.bezierCurveTo(c1x,fork,c2x,c2y,tipX,tipY); ctx.stroke();
      ctx.fillRect(tipX - 1, tipY - 1, 2, 2);
      // Decorative flow along the same cubic; never a transaction/activity signal.
      if(!reduced.matches&&n%4===0){
        const t=(phase*.085+n*.071)%1,u=1-t;
        const x=u*u*u*rootX+3*u*u*t*c1x+3*u*t*t*c2x+t*t*t*tipX;
        const y=u*u*u*baseY+3*u*u*t*fork+3*u*t*t*c2y+t*t*t*tipY;
        ctx.globalAlpha=.8;ctx.fillRect(x-1,y-1,2,2);
      }
    }
    for (let n=0;n<12;n++) { ctx.globalAlpha=.35;ctx.beginPath();ctx.moveTo(rootX,baseY);ctx.bezierCurveTo(rootX-n*3,baseY-h*.2,rootX-35+n*2,90,rootX-12+n*3,52+n*2);ctx.stroke(); }
    if (!compact) { ctx.globalAlpha = .75; ctx.strokeRect(split + 9, 33, w - split - 20, h * .17);
    const values = data.days.slice(-30).map(d => d.count ?? d.contributions ?? 0), max = Math.max(1, ...values);
    ctx.beginPath(); values.forEach((v, i) => { const x = split + 12 + i * (w - split - 27) / 29, y = 34 + h * .16 - v / max * h * .15; if (i) ctx.lineTo(x, y); else ctx.moveTo(x, y); }); ctx.stroke();
    const rows = [['API', data.connection], ['agents', data.agentCount ?? 'unavailable'], ['economy', 'simulation'], ['signing', 'disabled'], ['Mainnet', 'unavailable']];
    rows.forEach(([label, value], i) => { const y = h * .36 + i * 19; ctx.globalAlpha = .6; ctx.fillText(label, split + 10, y); ctx.textAlign = 'right'; ctx.fillText(String(value), w - 12, y); ctx.textAlign = 'left'; ctx.globalAlpha = .2; ctx.beginPath(); ctx.moveTo(split + 10, y + 5); ctx.lineTo(w - 12, y + 5); ctx.stroke(); });
    }
    ctx.globalAlpha = .6; ctx.fillText('root · Supa Agent / not runtime activity', 15, h - 12); if(!compact)ctx.fillText('decorative topology', split + 10, h - 12); ctx.globalAlpha = 1;
  }
  function drawEntity(time) {
    const view = fit(entity); if (!view) return;
    ascii(view.ctx, 0, 0, view.w, view.h * .8, time, true);
    view.ctx.font = '7px Plex,monospace'; view.ctx.globalAlpha = .6;
    view.ctx.fillText(`${worm ? 'worm: awake' : 'eye: tracking'} · ${input.slice(-8)}`, 12, view.h - 10); view.ctx.globalAlpha = 1;
  }
  function drawPointer() {
    const view=fit(tether);if(!view)return;
    tether.dataset.active=String(pointer.visible&&mode==='projects'&&!warpStart);
    if(tether.dataset.active!=='true')return;
    const box=entity.getBoundingClientRect(),x=pointer.x*innerWidth,y=pointer.y*innerHeight;
    const startX=box.left+box.width*.5,startY=box.top+box.height*.4;
    const {ctx}=view;ctx.globalAlpha=.6;ctx.lineWidth=1;
    ctx.setLineDash([5,5]);ctx.beginPath();ctx.moveTo(startX,startY);ctx.lineTo(x,y);ctx.stroke();
    ctx.setLineDash([]);ctx.strokeRect(x-9,y-6,18,12);
    tether.dataset.endpoints=[startX,startY,x,y].join(',');
  }
  function drawSpark() {
    const view = fit(spark); if (!view) return;
    const rows = data.days.slice(-56), max = Math.max(1, ...rows.map(d => d.count ?? 0));
    rows.forEach((d, i) => { const height = Math.max(1, (d.count ?? 0) / max * view.h); view.ctx.globalAlpha = .45 + height / view.h * .4; view.ctx.fillRect(i * view.w / rows.length, view.h - height, Math.max(1, view.w / rows.length - 1), height); });
  }
  function drawWarp(time) {
    const view = fit(transition); if (!view) return;
    const progress = Math.min(1, Math.max(0,(time - warpStart) / 2400)), { ctx, w, h } = view;
    const cx=w/2,cy=h/2,base=Math.min(w,h)*.315,outer=Math.hypot(w,h)*.65;
    const exit=Math.max(0,(progress-.72)/.28);
    const radius=base*(.94+.06*Math.min(1,progress/.3))+(outer-base)*exit**2;
    // Opaque full-screen stage: the editor must never show through the portal.
    ctx.globalAlpha=1;ctx.fillStyle=bg();ctx.fillRect(0,0,w,h);
    const field=ctx.createRadialGradient(cx,cy,radius,cx,cy,radius+base*.85);
    field.addColorStop(0,'rgba(216,212,180,.12)');field.addColorStop(1,'rgba(216,212,180,0)');
    ctx.fillStyle=field;ctx.fillRect(0,0,w,h);
    ctx.strokeStyle=ink();ctx.lineWidth=.85;
    for (let i = 0; i < 1500; i++) {
      const angle=i*2.3999632297,seed=(i*.754877666+progress*.52)%1;
      const distance=radius+8+seed**1.45*Math.max(0,outer-radius);
      const length=4+((i*.56984029)%1)*Math.min(45,h*.035);
      ctx.globalAlpha=.18+.42*(1-seed);
      ctx.beginPath();ctx.moveTo(cx+Math.cos(angle)*distance,cy+Math.sin(angle)*distance);
      ctx.lineTo(cx+Math.cos(angle)*(distance+length),cy+Math.sin(angle)*(distance+length));ctx.stroke();
    }
    ctx.globalAlpha=.1;ctx.lineWidth=8;ctx.beginPath();ctx.arc(cx,cy,radius+5,0,Math.PI*2);ctx.stroke();
    ctx.globalAlpha=1;ctx.fillStyle='#000';ctx.beginPath();ctx.arc(cx,cy,radius,0,Math.PI*2);ctx.fill();
    ctx.globalAlpha=.65;ctx.lineWidth=1;ctx.strokeStyle=ink();ctx.stroke();ctx.globalAlpha=1;
    transition.dataset.progress=progress.toFixed(3);
    if(portalBrand){
      const fadeIn=Math.min(1,progress/.13),fadeOut=Math.max(0,1-Math.max(0,progress-.72)/.18);
      portalBrand.style.setProperty('--portal-brand-opacity',(fadeIn*fadeOut).toFixed(3));
      portalBrand.style.setProperty('--portal-brand-scale',(.98+.02*fadeIn).toFixed(3));
      portalBrand.style.setProperty('--portal-logo-size',`${Math.max(72,Math.min(192,Math.round(Math.min(w,h)*.18/24)*24))}px`);
    }
    if (progress >= 1) finishWarp(true);
  }
  function finishWarp(completed) {
    warpStart=0;transition.classList.remove('active');
    const resolve=warpResolve;warpResolve=undefined;resolve?.(completed);
  }
  function draw(time = performance.now()) {
    const animateDiagram=mode==='projects'&&diagramVisible&&!diagramPaused&&!reduced.matches&&!warpStart;
    blueprint.dataset.motionState=reduced.matches?'reduced':diagramPaused?'paused':animateDiagram?'running':'offscreen';
    if(animateDiagram){
      if(diagramPrevious!==null)diagramTime+=Math.max(0,Math.min(100,time-diagramPrevious));
      drawBlueprint();
    }
    diagramPrevious=animateDiagram?time:null;
    if (mode === 'intro') drawIntro(time); else { drawEntity(time); if (worm) { const p = fit(spark); if (p) { p.ctx.fillStyle = ink(); p.ctx.fillRect(((time / 20) % p.w), 8, 3, 3); } } }
    drawPointer();
    if (warpStart) drawWarp(time);
  }
  function tick(time) { frame = 0; if (document.hidden) return; if(document.documentElement.dataset.motion!==(reduced.matches?'still':'running')){motionChange();return;} if (time - previous > 33) { draw(time); previous = time; } if (!reduced.matches || warpStart) frame = requestAnimationFrame(tick); }
  function schedule() { if (document.hidden) return; if (reduced.matches && !warpStart) draw(); else if (!frame) frame = requestAnimationFrame(tick); }
  function resize() { drawBlueprint(); drawSpark(); draw(); schedule(); }
  function pointerMove(e) { pointer.x = e.clientX / innerWidth; pointer.y = e.clientY / innerHeight; pointer.visible=e.pointerType!=='touch';schedule(); }
  function pointerLeave() { pointer.visible=false;schedule(); }
  function visibility() { introTime=null;diagramPrevious=null; if (document.hidden) { cancelAnimationFrame(frame); frame = 0; } else schedule(); }
  function updateDiagramControl(){diagramControl.disabled=reduced.matches;diagramControl.textContent=reduced.matches?'Diagram motion reduced':diagramPaused?'Play diagram':'Pause diagram';diagramControl.setAttribute('aria-pressed',String(!diagramPaused&&!reduced.matches));}
  function toggleDiagram(){diagramPaused=!diagramPaused;diagramPrevious=null;updateDiagramControl();schedule();}
  diagramControl.addEventListener('click',toggleDiagram);
  function motionChange() { document.documentElement.dataset.motion = reduced.matches ? 'still' : 'running';updateDiagramControl();diagramPrevious=null; if (reduced.matches) { cancelAnimationFrame(frame); frame = 0; if (warpResolve) finishWarp(true); } resize(); }
  const diagramObserver=new IntersectionObserver(entries=>{diagramVisible=entries[0].isIntersecting;diagramPrevious=null;schedule();});diagramObserver.observe(blueprint);
  const observer = new ResizeObserver(resize); [intro, blueprint, entity, spark].forEach(c => observer.observe(c));
  window.addEventListener('pointermove', pointerMove, { passive: true });document.documentElement.addEventListener('pointerleave',pointerLeave);window.addEventListener('blur',pointerLeave); window.addEventListener('resize', resize); document.addEventListener('visibilitychange', visibility); reduced.addEventListener('change', motionChange); motionChange();
  return {
    setMode(value) { mode = value; if(value==='intro'){introProgress=0;handBeat=-1;handKey='';}introTime=null;resize(); },
    setCode(text) { code=String(text).split('\n').slice(0,500);introProgress=0;introTime=null;handBeat=-1;handKey='';resize(); },
    setSelection(index) { selected = index; blueprint.dataset.selection = String(selected); drawBlueprint(); },
    setWorm(value) { worm = value; schedule(); },
    onInput(value) { input = value; schedule(); },
    redraw: resize,
    warp() { if (reduced.matches) return Promise.resolve(true); if (warpResolve) return Promise.resolve(false); return new Promise(resolve => { warpResolve = resolve; warpStart = performance.now(); transition.dataset.progress='0';drawWarp(warpStart);transition.classList.add('active');schedule(); }); },
    cancelWarp() { finishWarp(false); },
    destroy() { cancelAnimationFrame(frame); observer.disconnect();diagramObserver.disconnect();diagramControl.removeEventListener('click',toggleDiagram); window.removeEventListener('pointermove', pointerMove);document.documentElement.removeEventListener('pointerleave',pointerLeave);window.removeEventListener('blur',pointerLeave);tether.remove(); window.removeEventListener('resize', resize); document.removeEventListener('visibilitychange', visibility); reduced.removeEventListener('change', motionChange); finishWarp(false); },
  };
}
function format(value) { return Number(value).toLocaleString('en-US').replaceAll(',', "'"); }
