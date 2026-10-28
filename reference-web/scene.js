// Independently authored procedural scenes, informed by observed browser behavior.
// No reference scripts, filmed portrait or tracking code are included.
export function createScenes({ intro, blueprint, entity, transition, spark, reduced, data }) {
  let mode = 'intro', frame = 0, previous = 0, warpStart = 0, warpResolve, input = '', selected = 0, worm = false;
  const pointer = { x: .5, y: .5 };
  let introProgress = 0, introTime = null;
  const contexts = new Map();
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
    const columns = small ? 45 : 50, rows = small ? 42 : 52;
    const cw = width / columns, ch = height / rows;
    ctx.font = `${Math.max(3, ch * .83)}px Plex,monospace`;
    const px = (pointer.x - .5) * .25, py = (pointer.y - .5) * .15;
    for (let r = 0; r < rows; r++) for (let c = 0; c < columns; c++) {
      const nx = c / columns * 2 - 1, ny = r / rows * 2 - 1;
      const face = (nx - px) ** 2 / .64 + ny ** 2 / .9;
      if (face > 1) continue;
      const eyes = Math.exp(-(((Math.abs(nx - px) - .29) / .13) ** 2 + ((ny + .18 - py) / .075) ** 2));
      const nose = Math.exp(-(((nx - px) / .09) ** 2) - ((ny - .05) / .25) ** 2);
      const mouth = Math.exp(-(((nx - px) / .31) ** 2) - ((ny - .48) / .035) ** 2);
      const cheek = Math.max(0, 1 - face) * (.4 + .15 * Math.sin(c * .75 + r * .1));
      const blink = reduced.matches ? 1 : Math.sin(time / 1900) > .996 ? .1 : 1;
      const tone = Math.max(.04, cheek + eyes * blink + nose * .6 + mouth * .7);
      ctx.globalAlpha = Math.min(.85, tone * .75);
      const ramp = ' .,:;i1tfLCG08@';
      ctx.fillText(ramp[Math.min(ramp.length - 1, Math.floor(tone * 10))], x + c * cw, y + r * ch);
    }
    ctx.globalAlpha = 1;
  }
  // Procedural tracking illustration: pointer-driven geometry, never webcam data.
  function drawHands(ctx, w, h, time) {
    const scale = Math.min(1.25, w / 900), x = w * .5, y = h * .785;
    ctx.save(); ctx.translate(x, y); ctx.scale(scale, scale);
    ctx.font = '11px Plex,monospace'; ctx.globalAlpha = .2;
    const keys = ['esc  1 2 3 4 5 6 7 8 9 0 - =', '  q w e r t y u i o p [ ]', '   a s d f g h j k l ;', '    z x c v b n m , . /'];
    keys.forEach((text, row) => ctx.fillText(text, -150, row * 23));
    const connections = [[0,1,2,3,4],[0,5,6,7,8],[0,9,10,11,12],[0,13,14,15,16],[0,17,18,19,20],[5,9,13,17]];
    for (const [hand, sign] of [[0,-1],[1,1]]) {
      const hx = sign * 67 + (pointer.x - .5) * 18, hy = 65 + (pointer.y - .5) * 9;
      const points = [[hx,hy+83]];
      // Thumb and four articulated fingers, each with four visible landmarks.
      for(let finger=0;finger<5;finger++){
        const bend = reduced.matches ? 0 : Math.sin(time/1200+finger+hand)*2;
        const baseX=hx+(finger-2)*17, tipY=hy-40+Math.abs(finger-2)*7;
        for(let joint=0;joint<4;joint++){
          const t=joint/3;
          points.push([baseX+(finger===0?-sign*17*t:sign*2*t),hy+24+(tipY-hy-24)*t+bend*t]);
        }
      }
      ctx.globalAlpha=.42;ctx.lineWidth=.65;
      for(const path of connections){ctx.beginPath();path.forEach((id,i)=>{if(i)ctx.lineTo(...points[id]);else ctx.moveTo(...points[id]);});ctx.stroke();}
      ctx.globalAlpha=.72;for(const [px,py] of points)ctx.fillRect(px-1,py-1,2,2);
      ctx.globalAlpha=.36;ctx.strokeRect(hx-53,hy-45,106,132);
      ctx.font='8px Plex,monospace';ctx.globalAlpha=.6;
      ctx.fillText(`hand:${hand} · u ${(pointer.x*.2+.4+sign*.1).toFixed(5)}`,hx-55,hy-51);
    }
    ctx.restore();
  }
  function drawCodeFace(ctx,x,y,width,height,time) {
    const columns=48,rows=58,cw=width/columns,ch=height/rows;
    const lookX=(pointer.x-.5)*.16,lookY=(pointer.y-.5)*.08;
    ctx.save();ctx.font=`${ch*.88}px Plex,monospace`;
    for(let row=0;row<rows;row++){
      const ny=row/(rows-1)*2-1;
      // Head, cheeks and tapered jaw; glyphs use this implementation's public code.
      const halfWidth=ny<-.65?Math.sqrt(Math.max(0,1-((ny+.58)/.44)**2))*.75:ny>.36?.83-(ny-.36)*.65:.86;
      const text=(code[(row*3)%code.length].trim().replace(/\s+/g,' ')||'const frame = pointer.position;');
      for(let col=0;col<columns;col++){
        const nx=col/(columns-1)*2-1;
        if(Math.abs(nx)>halfWidth)continue;
        const eyes=Math.exp(-(((Math.abs(nx-lookX)-.3)/.15)**2)-((ny+.17-lookY)/.065)**2);
        const nose=Math.exp(-(((nx-lookX)/.105)**2)-((ny-.07)/.28)**2);
        const mouth=Math.exp(-(((nx-lookX)/.31)**2)-((ny-.48)/.045)**2);
        const surface=Math.max(0,1-(nx/Math.max(.1,halfWidth))**2)*(.18+.11*Math.sin(row*.7+col*.22));
        const blink=!reduced.matches&&Math.sin(time/1900)>.996?.15:1;
        ctx.globalAlpha=Math.min(.9,.07+surface+eyes*.58*blink+nose*.46+mouth*.38);
        ctx.fillStyle=col%11===0?'#71957d':ink();
        ctx.fillText(text[col%text.length],x+col*cw,y+row*ch);
      }
    }
    const eyeY=y+height*(.415+lookY*.5), eyeX=x+width*(.5+lookX*.5);
    ctx.strokeStyle=ink();ctx.fillStyle=ink();ctx.lineWidth=.6;ctx.globalAlpha=.55;
    ctx.strokeRect(eyeX-width*.23,eyeY-height*.045,width*.46,height*.09);
    ctx.fillRect(eyeX-1,eyeY-1,2,2);
    if(!reduced.matches){
      const targetX=pointer.x*innerWidth,targetY=pointer.y*innerHeight;
      ctx.globalAlpha=.3;ctx.setLineDash([3,4]);ctx.beginPath();ctx.moveTo(eyeX,eyeY);ctx.lineTo(targetX,targetY);ctx.stroke();ctx.setLineDash([]);
      ctx.strokeRect(targetX-9,targetY-6,18,12);
    }
    ctx.font='8px Plex,monospace';ctx.globalAlpha=.65;
    ctx.fillText(`pointer:u ${pointer.x.toFixed(6)}`,x+width*.8,y+height*.07);
    ctx.fillText(`pointer:v ${pointer.y.toFixed(6)}`,x+width*.8,y+height*.07+10);
    ctx.restore();
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
    let remaining=introProgress,revealedRows=0,typedCharacters=0,activeRow=0,activeColumn=0;
    ctx.save();ctx.beginPath();ctx.rect(left,top,width,bottom-top);ctx.clip();
    ctx.globalAlpha=.25;ctx.fillText('// — reference-web/app.js —',left+cell*3,top+font);
    for(let row=0;row<rows;row++){
      const limit=Math.max(1,Math.floor(width/cell)-5),full=code[row%code.length].slice(0,limit);
      const count=Math.min(full.length,Math.max(0,Math.floor(remaining))),started=remaining>=0,complete=remaining>=full.length+8;
      remaining-=full.length+8;
      if(complete)revealedRows++;
      typedCharacters+=count;
      if(!started)continue;
      activeRow=row;activeColumn=count;
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
    // A text-built entity overlays the single editor, with articulated hands below.
    const faceWidth=narrow?Math.min(145,w*.38):Math.min(290,w*.17);
    const faceX=left+width*.32-faceWidth/2+(pointer.x-.5)*width*.12,faceY=top+font*2+(pointer.y-.5)*h*.06;
    drawCodeFace(ctx,faceX,faceY,faceWidth,faceWidth*1.35,time);
    drawHands(ctx,w,h,time);
    ctx.globalAlpha=1;
  }
  function drawBlueprint() {
    const view = fit(blueprint); if (!view) return;
    const { ctx, w, h } = view, compact = w < 400, split = compact ? w : w * .62, rootX = w * (compact ? .3 : .15), baseY = h * (compact ? .88 : .92);
    ctx.font = '9px Plex,monospace'; ctx.globalAlpha = .7;
    ctx.fillText(`HC / ${data.contributions} contributions · public snapshot`, 9, 16);
    if(!compact) ctx.fillText('section A—A', split + 10, 24);
    ctx.lineWidth = .5;
    for (let r = 0; r < 45; r++) { const y = 32 + r * (h - 45) / 45; ctx.globalAlpha = .1; ctx.beginPath(); ctx.moveTo(15, y); ctx.lineTo(split - 10, y); ctx.stroke(); }
    ctx.globalAlpha = .7;
    for (let n = 0; n < 35; n++) {
      const fork = baseY - (n / 35) * h * .45;
      const side = n % 6 === 0 ? -.3 : .7;
      const tipX = Math.min(split - 13, rootX + (n % 6 === 0 ? 20 : split * .58)) - Math.sin(n * .9) * 8;
      const tipY = 50 + n * (h - 95) / 41;
      ctx.globalAlpha = .25 + n / 70; ctx.beginPath(); ctx.moveTo(rootX, baseY);
      ctx.bezierCurveTo(rootX - n * side, fork, rootX + (tipX - rootX) * .45, tipY + n, tipX, tipY); ctx.stroke();
      ctx.fillRect(tipX - 1, tipY - 1, 2, 2);
    }
    for (let n=0;n<12;n++) { ctx.globalAlpha=.35;ctx.beginPath();ctx.moveTo(rootX,baseY);ctx.bezierCurveTo(rootX-n*3,baseY-h*.2,rootX-35+n*2,90,rootX-12+n*3,52+n*2);ctx.stroke(); }
    if (!compact) { ctx.globalAlpha = .75; ctx.strokeRect(split + 9, 33, w - split - 20, h * .17);
    const values = data.days.slice(-30).map(d => d.count ?? d.contributions ?? 0), max = Math.max(1, ...values);
    ctx.beginPath(); values.forEach((v, i) => { const x = split + 12 + i * (w - split - 27) / 29, y = 34 + h * .16 - v / max * h * .15; if (i) ctx.lineTo(x, y); else ctx.moveTo(x, y); }); ctx.stroke();
    const rows = [['repositories', data.repositoryCount], ['stars', format(data.stars)], ['followers', format(data.followers)], ['contributions', format(data.contributions)], ['snapshot', data.fetchedAt.slice(0, 10)]];
    rows.forEach(([label, value], i) => { const y = h * .36 + i * 19; ctx.globalAlpha = .6; ctx.fillText(label, split + 10, y); ctx.textAlign = 'right'; ctx.fillText(String(value), w - 12, y); ctx.textAlign = 'left'; ctx.globalAlpha = .2; ctx.beginPath(); ctx.moveTo(split + 10, y + 5); ctx.lineTo(w - 12, y + 5); ctx.stroke(); });
    }
    ctx.globalAlpha = .6; ctx.fillText(`root · ${data.account}`, 15, h - 12); if(!compact)ctx.fillText('generated geometry', split + 10, h - 12); ctx.globalAlpha = 1;
  }
  function drawEntity(time) {
    const view = fit(entity); if (!view) return;
    ascii(view.ctx, 0, 0, view.w, view.h * .8, time, true);
    view.ctx.font = '7px Plex,monospace'; view.ctx.globalAlpha = .6;
    view.ctx.fillText(`${worm ? 'worm: awake' : 'eye: tracking'} · ${input.slice(-8)}`, 12, view.h - 10); view.ctx.globalAlpha = 1;
  }
  function drawSpark() {
    const view = fit(spark); if (!view) return;
    const rows = data.days.slice(-56), max = Math.max(1, ...rows.map(d => d.count ?? 0));
    rows.forEach((d, i) => { const height = Math.max(1, (d.count ?? 0) / max * view.h); view.ctx.globalAlpha = .45 + height / view.h * .4; view.ctx.fillRect(i * view.w / rows.length, view.h - height, Math.max(1, view.w / rows.length - 1), height); });
  }
  function drawWarp(time) {
    const view = fit(transition); if (!view) return;
    const progress = Math.min(1, (time - warpStart) / 1700), { ctx, w, h } = view;
    ctx.fillStyle = bg(); ctx.globalAlpha = 1 - Math.abs(progress - .5); ctx.fillRect(0, 0, w, h); ctx.strokeStyle = ink();
    for (let i = 0; i < 220; i++) {
      const angle = i * 2.399963, radius = (i % 31) / 31 * Math.max(w, h) * (progress + .02);
      const x = w * pointer.x + Math.cos(angle) * radius, y = h * pointer.y + Math.sin(angle) * radius;
      ctx.globalAlpha = Math.sin(progress * Math.PI) * .6; ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + Math.cos(angle) * progress * 60, y + Math.sin(angle) * progress * 60); ctx.stroke();
    }
    ctx.globalAlpha = 1;
    if (progress >= 1) { warpStart = 0; transition.classList.remove('active'); warpResolve?.(); warpResolve = undefined; }
  }
  function draw(time = performance.now()) {
    if (mode === 'intro') drawIntro(time); else { drawEntity(time); if (worm) { const p = fit(spark); if (p) { p.ctx.fillStyle = ink(); p.ctx.fillRect(((time / 20) % p.w), 8, 3, 3); } } }
    if (warpStart) drawWarp(time);
  }
  function tick(time) { frame = 0; if (document.hidden) return; if(document.documentElement.dataset.motion!==(reduced.matches?'still':'running')){motionChange();return;} if (time - previous > 33) { draw(time); previous = time; } if (!reduced.matches || warpStart) frame = requestAnimationFrame(tick); }
  function schedule() { if (document.hidden) return; if (reduced.matches && !warpStart) draw(); else if (!frame) frame = requestAnimationFrame(tick); }
  function resize() { drawBlueprint(); drawSpark(); draw(); schedule(); }
  function pointerMove(e) { pointer.x = e.clientX / innerWidth; pointer.y = e.clientY / innerHeight; schedule(); }
  function visibility() { introTime=null; if (document.hidden) { cancelAnimationFrame(frame); frame = 0; } else schedule(); }
  function motionChange() { document.documentElement.dataset.motion = reduced.matches ? 'still' : 'running'; if (reduced.matches) { cancelAnimationFrame(frame); frame = 0; if (warpResolve) { warpResolve(); warpResolve = undefined; warpStart = 0; transition.classList.remove('active'); } } resize(); }
  const observer = new ResizeObserver(resize); [intro, blueprint, entity, spark].forEach(c => observer.observe(c));
  window.addEventListener('pointermove', pointerMove, { passive: true }); window.addEventListener('resize', resize); document.addEventListener('visibilitychange', visibility); reduced.addEventListener('change', motionChange); motionChange();
  return {
    setMode(value) { mode = value; introTime=null; resize(); },
    setCode(text) { code=String(text).split('\n').slice(0,500);introProgress=0;introTime=null;resize(); },
    setSelection(index) { selected = index; blueprint.dataset.selection = String(selected); drawBlueprint(); },
    setWorm(value) { worm = value; schedule(); },
    onInput(value) { input = value; schedule(); },
    redraw: resize,
    warp() { if (reduced.matches) return Promise.resolve(); if (warpResolve) return Promise.resolve(); return new Promise(resolve => { warpResolve = resolve; warpStart = performance.now(); transition.classList.add('active'); schedule(); }); },
    destroy() { cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener('pointermove', pointerMove); window.removeEventListener('resize', resize); document.removeEventListener('visibilitychange', visibility); reduced.removeEventListener('change', motionChange); warpResolve?.(); },
  };
}
function format(value) { return Number(value).toLocaleString('en-US').replaceAll(',', "'"); }
