// Independently authored procedural scenes, informed by observed browser behavior.
// No reference scripts, filmed portrait or tracking code are included.
export function createScenes({ intro, blueprint, entity, transition, spark, reduced, data }) {
  let mode = 'intro', frame = 0, previous = 0, warpStart = 0, warpResolve, input = '', selected = 0, worm = false;
  const pointer = { x: .5, y: .5 }, born = performance.now();
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
  function drawHands(ctx, w, h, time) {
    const scale = Math.min(1, w / 650), x = w * .5, y = h * .83;
    ctx.save(); ctx.translate(x, y); ctx.scale(scale, scale);
    ctx.font = '9px Plex,monospace'; ctx.globalAlpha = .18;
    ['1 2 3 4 5 6 7 8 9 0 - =', ' q w e r t y u i o p [ ]', '  a s d f g h j k l ;', '   z x c v b n m , . /'].forEach((s, i) => ctx.fillText(s, -150, i * 25));
    ctx.globalAlpha = .55; ctx.lineWidth = .55;
    for (const sign of [-1, 1]) {
      const handX = sign * (72 + (pointer.x - .5) * 18), handY = 65 + sign * Math.sin(time / 1100) * 2;
      const wrist = [handX + sign * 8, handY + 58];
      for (let finger = 0; finger < 5; finger++) {
        const tip = [handX + (finger - 2) * 15, handY - 50 + Math.abs(finger - 2) * 7 + Math.sin(time / 700 + finger) * 4];
        const joint = [(tip[0] + wrist[0]) / 2, handY + 3];
        ctx.beginPath(); ctx.moveTo(...wrist); ctx.lineTo(...joint); ctx.lineTo(...tip); ctx.stroke();
        for (const p of [wrist, joint, tip]) { ctx.fillRect(p[0] - 1, p[1] - 1, 2, 2); }
      }
    }
    ctx.globalAlpha = .5; ctx.fillText(input.slice(-28) || "['input'] · pointer / keys", -145, -18); ctx.restore();
  }
  function drawIntro(time) {
    const view = fit(intro); if (!view) return;
    const { ctx, w, h } = view, line=13, cell=7.2;
    const narrow=w<700, onePane=reduced.matches||narrow, width=Math.min(720,w-112), left=(w-width)/2;
    const rows=Math.max(15,Math.floor((onePane?h-210:h*.58)/line));
    ctx.font='12px Plex,monospace';
    const shift=reduced.matches?0:Math.floor((time-born)/5200)%12;
    ctx.save();ctx.beginPath();ctx.rect(Math.max(0,left-32),110,width+64,rows*line+14);ctx.clip();
    for(let pane=0;pane<(onePane?1:2);pane++){
      const gutter=left+(pane*width/2)-(narrow?28:22), textX=left+(onePane&&narrow?14:0)+pane*width/2;
      for(let row=0;row<rows;row++){
        const value=code[(row+shift+pane*3)%code.length];
        ctx.fillStyle=ink();ctx.globalAlpha=.28;ctx.fillText(String(row+1+shift).padStart(2),gutter,126+row*line);
        ctx.globalAlpha=.31+(row%5)*.025;
        if(/^(import|const|export|function|let)/.test(value))ctx.fillStyle='#71957d';
        const limit=Math.floor((onePane?width:width/2)/cell)-(narrow?1:0);
        ctx.fillText(value.slice(0,limit),textX,126+row*line);
      }
    }
    ctx.restore();ctx.globalAlpha=1;ctx.fillStyle=ink();
    if(!onePane){
      ascii(ctx,left+width*.28,70,190,210,time);
      ctx.save();ctx.translate(left,0);drawHands(ctx,width,h*.94,time);ctx.restore();
    }
    if(!reduced.matches){const cx=left+width*(.5+(pointer.x-.5)*.2),cy=onePane?h*.3:h*.24;ctx.globalAlpha=.5;ctx.fillRect(Math.round(cx/cell)*cell,Math.round(cy/line)*line,cell,line);ctx.globalAlpha=1;}
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
  function visibility() { if (document.hidden) { cancelAnimationFrame(frame); frame = 0; } else schedule(); }
  function motionChange() { document.documentElement.dataset.motion = reduced.matches ? 'still' : 'running'; if (reduced.matches) { cancelAnimationFrame(frame); frame = 0; if (warpResolve) { warpResolve(); warpResolve = undefined; warpStart = 0; transition.classList.remove('active'); } } resize(); }
  const observer = new ResizeObserver(resize); [intro, blueprint, entity, spark].forEach(c => observer.observe(c));
  window.addEventListener('pointermove', pointerMove, { passive: true }); window.addEventListener('resize', resize); document.addEventListener('visibilitychange', visibility); reduced.addEventListener('change', motionChange); motionChange();
  return {
    setMode(value) { mode = value; resize(); },
    setCode(text) { code=String(text).split('\n').slice(0,500);resize(); },
    setSelection(index) { selected = index; blueprint.dataset.selection = String(selected); drawBlueprint(); },
    setWorm(value) { worm = value; schedule(); },
    onInput(value) { input = value; schedule(); },
    redraw: resize,
    warp() { if (reduced.matches) return Promise.resolve(); if (warpResolve) return Promise.resolve(); return new Promise(resolve => { warpResolve = resolve; warpStart = performance.now(); transition.classList.add('active'); schedule(); }); },
    destroy() { cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener('pointermove', pointerMove); window.removeEventListener('resize', resize); document.removeEventListener('visibilitychange', visibility); reduced.removeEventListener('change', motionChange); warpResolve?.(); },
  };
}
function format(value) { return Number(value).toLocaleString('en-US').replaceAll(',', "'"); }
