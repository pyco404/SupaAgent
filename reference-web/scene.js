import { navigation } from './platform.js';
// Presentation copy of the complete function; source parity is checked in browser tests.
export const RUNTIME_EXCERPT = Object.freeze({
  file: 'crates/runtime/src/lib.rs',
  firstLine: 108,
  title: 'cycle_inner · excerpt',
  code: String.raw`async fn cycle_inner(
    &self,
    id: Uuid,
    scheduled: bool,
    requested: Option<Plan>,
) -> Result<Uuid> {
    let cycle = self
        .store
        .begin_cycle_for(id, scheduled, self.worker)
        .await?;
    tracing::info!(agent_id=%id,cycle_id=%cycle,"cycle_started");
    let outcome =
        tokio::time::timeout(Duration::from_secs(180), self.run(id, cycle, requested))
            .await
            .unwrap_or(Err(Error::Timeout));
    if let Err(e) = self.store.finish(id, cycle, outcome.as_ref().err()).await
        && (!matches!(e, Error::Stale) || outcome.is_ok())
    {
        return Err(e);
    }
    match &outcome {
        Ok(()) => tracing::info!(agent_id=%id,cycle_id=%cycle,"cycle_completed"),
        Err(e) => tracing::warn!(agent_id=%id,cycle_id=%cycle,error_code=%e,"cycle_failed"),
    }
    outcome?;
    Ok(cycle)
}`,
});
const RUST_KEYWORDS = /^(?:as|async|await|break|const|continue|crate|dyn|else|enum|false|fn|for|if|impl|in|let|loop|match|mod|move|mut|pub|ref|return|self|Self|static|struct|super|trait|true|type|unsafe|use|where|while)$/;
export function rustTokens(line) {
  return line.split(/(\/\/.*$|"(?:\\.|[^"\\])*"?|'(?:\\.|[^'\\])'|\b(?:[A-Za-z_]\w*!|[A-Za-z_]\w*|\d+)\b!?)/g).filter(Boolean).map(text => ({
    text,
    kind: text.startsWith('//') ? 'comment' : /^["']/.test(text) ? 'string' : RUST_KEYWORDS.test(text) ? 'keyword' : /^\d+$/.test(text) ? 'number' : text.endsWith('!') ? 'macro' : /^[A-Z]/.test(text) ? 'type' : 'plain',
  }));
}
// Wrapping only changes presentation; columns still refer to the original Rust line.
export function rustEditorRows(code, limit, firstLine = RUNTIME_EXCERPT.firstLine) {
  return code.flatMap((text, index) => {
    const rows = [], tokens = rustTokens(text);
    let column = 0;
    do {
      rows.push({ text: text.slice(column, column + limit), column, sourceLine: firstLine + index, tokens });
      column += limit;
    } while (column < text.length);
    return rows;
  });
}

// Independently authored procedural scenes, informed by observed browser behavior.
// No reference scripts, filmed portrait or tracking code are included.
export function createScenes({ intro, blueprint, entity, transition, spark, reduced, data }) {
  let mode = 'intro', frame = 0, previous = 0, warpStart = 0, warpResolve, input = '', selected = 0, worm = false;
  const pointer = { x: .5, y: .5, visible: false, movedAt: -Infinity };
  let pointerIdleTimer=0,typingTarget=null,introTarget=null,terminalProgress=0,terminalCharacter='',terminalTyping=false;
  const workspaceHands=document.querySelector('.workspace-hands');
  const tether = document.createElement('canvas');
  tether.className='pointer-tether';tether.setAttribute('aria-hidden','true');
  tether.style.cssText='position:fixed;inset:0;width:100%;height:100%;pointer-events:none;z-index:8';
  document.body.append(tether);
  let introProgress = 0, introTime = null, handBeat = -1, handKey = "";
  let diagramTime=0,diagramPrevious=null,diagramVisible=false,diagramPaused=false;
  const diagramControl=document.querySelector('.diagram-motion');
  const contexts = new Map();
  const portalBrand = document.querySelector('.portal-brand');
  let savedLifecycle='unavailable',lifecycleChangedAt=-Infinity,entityDirty=true,entityPaintKey='';
  const code = RUNTIME_EXCERPT.code.split('\n');
  let editorLayoutKey = '', editorRows = [];
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
    // One subpixel code-row slip for 80ms every 37s; never affects terminal text.
    const slip=small&&!reduced.matches&&time%37000>34800&&time%37000<34880?.14:0;
    for(let row=0;row<30;row++)for(let column=0;column<48;column++){
      ctx.globalAlpha=.35+.25*((row+column)%4)/3;
      ctx.fillText(source[(row*48+column+offset)%source.length],column*.5+(row===12?slip:0),row*.8);
    }
    ctx.restore();
  }
  // Decorative hands share the editor's typing clock; no camera or key capture.
  function drawHands(ctx, w, h, typing, character, surface=intro, compact=false) {
    const mouseActive=pointer.visible&&performance.now()-pointer.movedAt<700;
    const progress=compact?terminalProgress:introProgress;
    const width=Math.min(700,(w-40)/1.28,compact?h*3.1:h*.8),pitch=width/15,rowPitch=pitch*.68,keyHeight=pitch*.54;
    const left=(w-width)/2,height=rowPitch*4.65,top=compact?Math.max(8,h-height-pitch*.42-8):h*.755;
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
    const beat=typing?Math.floor(progress/5):-1;
    if(beat!==handBeat){handBeat=beat;handKey=aliases[character]||character.toLowerCase();}
    const active=typing?byKey.get(handKey):null,pulse=typing?Math.sin((progress/5%1)*Math.PI):0;
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
      const mouseHand=hand===1&&mouseActive,mouseX=w/2+width*.58;
      ctx.save();
      if(mouseHand&&!reduced.matches){
        // Mirror real pointer travel within the keyboard's existing margins.
        const dx=Math.max(-.5,Math.min(.5,pointer.x-.5))*pitch*.8;
        const dy=Math.max(-.5,Math.min(.5,pointer.y-.5))*pitch*.6;
        ctx.translate(dx,Math.min(dy,h-3-(top+height+pitch*.36)));
      }
      const wristX=mouseHand?mouseX:w/2+sign*width*.145,wristY=top+height+pitch*.36,points=[[wristX,wristY]];
      if(mouseHand){
        ctx.globalAlpha=.45;ctx.lineWidth=.8;ctx.beginPath();ctx.roundRect(mouseX-pitch*.7,top+pitch*.65,pitch*1.4,pitch*2.3,pitch*.7);ctx.stroke();
        ctx.beginPath();ctx.moveTo(mouseX,top+pitch*.65);ctx.lineTo(mouseX,top+pitch*1.6);ctx.stroke();
      }
      for(let finger=0;finger<5;finger++){
        const rest=byKey.get(home[finger]),press=!mouseHand&&active&&hand===activeHand&&finger===activeFinger;
        const tipX=mouseHand?mouseX+[-.7,-.35,0,.3,.6][finger]*pitch:press?active.cx:rest.cx+(finger===0?sign*pitch*.65:0);
        const tipY=mouseHand?top+[2.1,.95,.85,1.05,1.6][finger]*pitch:(press?active.cy:rest.cy)-pitch*.18+(press?pulse*pitch*.23:0);
        const baseX=wristX+((mouseHand?tipX:rest.cx)-wristX)*.65,baseY=top+height*.85;
        for(let joint=0;joint<4;joint++){
          const t=joint/3;
          points.push([baseX+(tipX-baseX)*t,baseY+(tipY-baseY)*t-(press?Math.sin(t*Math.PI)*pulse*pitch*.12:0)]);
        }
      }
      ctx.strokeStyle=ink();ctx.fillStyle=ink();ctx.globalAlpha=.6;ctx.lineWidth=.8;
      for(const path of connections){ctx.beginPath();path.forEach((id,i)=>{if(i)ctx.lineTo(...points[id]);else ctx.moveTo(...points[id]);});ctx.stroke();}
      ctx.globalAlpha=.85;for(const [px,py] of points)ctx.fillRect(px-1,py-1,2,2);
      ctx.restore();
    }
    ctx.restore();
    surface.dataset.handState=typing?'typing':'idle';surface.dataset.rightHand=mouseActive?'mouse':'keyboard';surface.dataset.leftHand='keyboard';
    surface.dataset.keyboardBounds=[left,top,width,height+pitch*.42].map(n=>n.toFixed(2)).join(',');
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
    const limit=Math.max(1,Math.floor(width/cell)-5),layoutKey=String(limit);
    if(editorLayoutKey!==layoutKey){editorLayoutKey=layoutKey;editorRows=rustEditorRows(code,limit);}
    let remaining=introProgress,revealedRows=0,typedCharacters=0,activeRow=0,activeColumn=0,typedKey=" ";
    const revealed=editorRows.map((row,index)=>{
      const count=Math.min(row.text.length,Math.max(0,Math.floor(remaining))),started=remaining>=0,complete=remaining>=row.text.length+8;
      remaining-=row.text.length+8;
      if(complete)revealedRows++;
      typedCharacters+=count;
      if(started){activeRow=index;activeColumn=count;if(count>0)typedKey=row.text[count-1];}
      return {...row,count,started,complete};
    });
    const complete=revealedRows===editorRows.length;
    const scrollRow=reduced.matches?0:Math.max(0,activeRow-rows+1);
    ctx.save();ctx.beginPath();ctx.rect(left,top,width,bottom-top);ctx.clip();
    ctx.globalAlpha=.25;ctx.fillText('// lib.rs · '+RUNTIME_EXCERPT.title,left+cell*4,top+font);
    for(let row=scrollRow;row<Math.min(editorRows.length,scrollRow+rows);row++){
      const {column,sourceLine,tokens,count,started,complete:lineComplete}=revealed[row];
      if(!started)continue;
      const baseline=first+(row-scrollRow)*line;
      if(!lineComplete){ctx.globalAlpha=.7;ctx.fillStyle='#161615';ctx.fillRect(left,baseline-font,width,line);}
      ctx.fillStyle=ink();ctx.globalAlpha=.28;ctx.fillText(column?'':String(sourceLine).padStart(3),left,baseline);
      const textX=left+cell*4;
      let tx=textX,offset=0;
      for(const token of tokens){
        const start=Math.max(column,offset),end=Math.min(column+count,offset+token.text.length);
        if(end>start){
          const text=token.text.slice(start-offset,end-offset);
          ctx.globalAlpha=token.kind==='comment'?.32:token.kind==='string'?.56:.42;
          ctx.fillStyle=['keyword','macro'].includes(token.kind)?'#71957d':token.kind==='number'?'#cd796e':ink();
          ctx.fillText(text,tx,baseline);tx+=ctx.measureText(text).width;
        }
        offset+=token.text.length;
      }
      if(!lineComplete&&!reduced.matches&&Math.floor(time/550)%2===0){ctx.fillStyle=ink();ctx.globalAlpha=.7;ctx.fillRect(textX+count*cell,baseline-font,cell,line);}
    }
    ctx.restore();
    intro.dataset.revealedRows=String(revealedRows);intro.dataset.typedCharacters=String(typedCharacters);intro.dataset.introState=complete?'complete':'typing';
    intro.dataset.language='rust';intro.dataset.sourceFile=RUNTIME_EXCERPT.file;intro.dataset.visibleFirstLine=String(editorRows[scrollRow]?.sourceLine??RUNTIME_EXCERPT.firstLine);
    const status=document.querySelector('.intro-modes'),label=document.querySelector('.mode-label'),position=document.querySelector('.intro-position');
    const active=editorRows[activeRow],state=complete?'NORMAL':'INSERT',cursor=`${active.sourceLine}:${active.column+activeColumn+1}`;
    if(status)status.dataset.mode=state;
    if(label&&label.textContent!==state)label.textContent=state;
    if(position&&position.textContent!==cursor)position.textContent=cursor;
    introTarget=!reduced.matches&&!complete?{x:left+cell*4+activeColumn*cell,y:first+(activeRow-scrollRow)*line-font*.35}:null;
    drawHands(ctx,w,h,!reduced.matches&&!complete,typedKey);
    ctx.globalAlpha=1;
  }
  function drawAbout({ctx,w,h},phase) {
    // A conceptual identity plate, not a measurement of connected services.
    const cx=w/2,cy=h*.43,r=Math.min(w*.2,h*.25),small=w<400;
    ctx.font='11px Plex,monospace';ctx.globalAlpha=.75;
    ctx.fillText('SUPA / identity & boundaries',9,16);
    ctx.lineWidth=.6;
    for(let i=0;i<3;i++){
      const radius=r+9+i*8;
      ctx.globalAlpha=.18+i*.1;ctx.beginPath();
      for(let side=0;side<4;side++){
        const start=side*Math.PI/2+.12+phase*.12*(i%2?-1:1),end=start+Math.PI/2-.24;
        ctx.arc(cx,cy,radius,start,end);
      }
      ctx.stroke();
    }
    // Abstract processor schematic; no mascot or inferred runtime activity.
    const core=r*.52;
    ctx.globalAlpha=.6;ctx.strokeRect(cx-core,cy-core,core*2,core*2);
    ctx.globalAlpha=.22;ctx.strokeRect(cx-core*.78,cy-core*.78,core*1.56,core*1.56);
    for(let side=0;side<4;side++){
      ctx.save();ctx.translate(cx,cy);ctx.rotate(side*Math.PI/2);
      for(let pin=0;pin<4;pin++){
        const offset=(pin-1.5)*core*.38;
        ctx.globalAlpha=.45;ctx.beginPath();ctx.moveTo(offset,-core);ctx.lineTo(offset,-core*1.35);ctx.stroke();
        ctx.globalAlpha=.7;ctx.fillRect(offset-1,-core*1.35-1,2,2);
      }
      ctx.restore();
    }
    ctx.globalAlpha=.35;ctx.beginPath();
    ctx.moveTo(cx,cy-core*.56);ctx.lineTo(cx+core*.56,cy);ctx.lineTo(cx,cy+core*.56);ctx.lineTo(cx-core*.56,cy);ctx.closePath();ctx.stroke();
    ctx.font='8px Plex,monospace';ctx.textAlign='center';ctx.globalAlpha=.8;ctx.fillText('CORE',cx,cy+3);
    ctx.font='11px Plex,monospace';ctx.textAlign='left';
    const labels=[['MEMORY',12,cy-r*.55,-1],['GOALS',w-12,cy-r*.55,1],['POLICY',12,cy+r*.65,-1],['LIMITS',w-12,cy+r*.65,1]];
    for(const [label,x,y,side] of labels){
      const edge=cx+side*(r+28),end=x-side*(small?49:66);
      ctx.globalAlpha=.4;ctx.beginPath();ctx.moveTo(edge,cy+Math.sign(y-cy)*r*.45);ctx.lineTo(end,y-4);ctx.stroke();
      ctx.globalAlpha=.8;ctx.textAlign=side<0?'left':'right';ctx.fillText(label,x,y);
    }
    ctx.textAlign='center';
    const base=h*.78,gap=Math.min(140,w*.31),box=Math.min(90,w*.25);
    ctx.globalAlpha=.3;ctx.beginPath();ctx.moveTo(cx-gap,base);ctx.lineTo(cx+gap,base);ctx.stroke();
    ['RUST','POSTGRES','API'].forEach((label,i)=>{
      const x=cx+(i-1)*gap;
      ctx.globalAlpha=1;ctx.fillStyle=bg();ctx.fillRect(x-box/2,base-12,box,24);ctx.fillStyle=ink();
      ctx.globalAlpha=.5;ctx.strokeRect(x-box/2,base-12,box,24);
      ctx.globalAlpha=.85;ctx.fillText(label,x,base+4);
    });
    ctx.textAlign='left';ctx.globalAlpha=.65;
    ctx.fillText('Conceptual foundation / not live activity',12,h-12);ctx.globalAlpha=1;
  }
  function drawBlueprint() {
    const view = fit(blueprint); if (!view) return;
    const { ctx, w, h } = view, compact = w < 400, split = compact ? w : w * .62, rootX = w * (compact ? .3 : .15), baseY = h * (compact ? .88 : .92);
    const phase=reduced.matches?0:diagramTime/1000;
    blueprint.dataset.motionPhase=String(phase);
    const about=navigation[selected]?.id==='about';
    blueprint.dataset.concept=about?'identity':'architecture';
    blueprint.setAttribute('aria-label',about?'Conceptual Supa identity, memory, goals, policy and limits over Rust, PostgreSQL and API; not live agent activity':'Decorative architecture branches; not live agent activity');
    if(about){drawAbout(view,phase);return;}
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
    // The code texture changes every 1.8s, not every frame. Preserve precise redraws
    // for lifecycle settling, rare interference, input, palette and resize events.
    const settling=!reduced.matches&&time-lifecycleChangedAt<800;
    const paintKey=[reduced.matches?0:Math.floor(time/1800),!reduced.matches&&time%37000>34800&&time%37000<34880,savedLifecycle,worm,input.slice(-8),settling?Math.floor(time/33):0].join('|');
    if(!entityDirty&&paintKey===entityPaintKey)return;
    const view = fit(entity); if (!view) return;
    entityPaintKey=paintKey;entityDirty=false;
    ascii(view.ctx, 0, 0, view.w, view.h * .8, time, true);
    const ctx=view.ctx,known=savedLifecycle!=='unavailable';
    const emphasis={active:.65,created:.48,paused:.38,suspended:.28,terminated:.18,unavailable:.18}[savedLifecycle];
    const settle=reduced.matches?0:Math.max(0,1-(time-lifecycleChangedAt)/800)*.12;
    ctx.save();ctx.strokeStyle=savedLifecycle==='active'?getComputedStyle(document.documentElement).getPropertyValue('--green').trim():ink();
    ctx.globalAlpha=emphasis+settle;ctx.lineWidth=.6;
    const inset=6,bottom=view.h*.8;
    for(const [x,y,dx,dy]of [[inset,inset,1,1],[view.w-inset,inset,-1,1],[inset,bottom,1,-1],[view.w-inset,bottom,-1,-1]]){
      ctx.beginPath();ctx.moveTo(x+dx*7,y);ctx.lineTo(x,y);ctx.lineTo(x,y+dy*7);ctx.stroke();
    }
    ctx.restore();entity.dataset.lifecycle=savedLifecycle;entity.dataset.lifecycleSource=known?'saved-record':'unavailable';
    ctx.font='7px Plex,monospace';ctx.globalAlpha=.65;ctx.fillText(`${view.w<140?'saved':'saved identity'}: ${savedLifecycle}`,12,view.h-21);ctx.globalAlpha=1;
    view.ctx.font = '7px Plex,monospace'; view.ctx.globalAlpha = .6;
    view.ctx.fillText(`${worm ? 'worm: awake' : 'eye: tracking'} · ${input.slice(-8)}`, 12, view.h - 10); view.ctx.globalAlpha = 1;
  }
  function drawPointer() {
    const view=fit(tether);if(!view)return;
    tether.dataset.active=String(pointer.visible&&!warpStart);
    tether.dataset.typingActive='false';
    if(warpStart)return;
    const anchor=mode==='intro'?document.querySelector('.intro-mascot'):entity;
    const box=anchor.getBoundingClientRect(),startX=box.left+box.width*.5,startY=box.top+box.height*.4;
    const {ctx}=view;ctx.globalAlpha=.6;ctx.lineWidth=1;
    function connect(x,y){ctx.setLineDash([5,5]);ctx.beginPath();ctx.moveTo(startX,startY);ctx.lineTo(x,y);ctx.stroke();ctx.setLineDash([]);ctx.strokeRect(x-9,y-6,18,12);}
    if(pointer.visible){
      const x=pointer.x*innerWidth,y=pointer.y*innerHeight;connect(x,y);
      tether.dataset.endpoints=[startX,startY,x,y].join(',');
    }
    let target=mode==='intro'?introTarget:null;
    if(mode==='projects'&&terminalTyping&&typingTarget?.text.isConnected&&typingTarget.offset>0){
      const {text,offset}=typingTarget,range=document.createRange(),limit=Math.min(offset,text.length);
      if(limit){range.setStart(text,limit-1);range.setEnd(text,limit);const rect=range.getBoundingClientRect(),buffer=document.querySelector('.buffer').getBoundingClientRect();
        if(rect.height&&rect.top>=buffer.top&&rect.bottom<=buffer.bottom)target={x:rect.right,y:rect.top+rect.height/2};}
    }
    const command=document.querySelector('.command-input');
    if(document.activeElement===command){const rect=command.getBoundingClientRect();ctx.font=getComputedStyle(command).font;target={x:Math.min(rect.right-4,rect.left+ctx.measureText(command.value.slice(0,command.selectionStart)).width),y:rect.top+rect.height/2};}
    if(target&&target.y>0&&target.y<innerHeight){
      ctx.globalAlpha=.35;connect(target.x,target.y);tether.dataset.typingActive='true';tether.dataset.typingEndpoints=[startX,startY,target.x,target.y].join(',');
    }
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
      const fadeIn=Math.min(1,Math.max(0,(progress-.035)/.18)),fadeOut=Math.max(0,1-Math.max(0,progress-.72)/.18);
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
    if (mode === 'intro') drawIntro(time); else { drawEntity(time);const hands=fit(workspaceHands);if(hands)drawHands(hands.ctx,hands.w,hands.h,terminalTyping&&!reduced.matches,terminalCharacter,workspaceHands,true); if (worm) { const p = fit(spark); if (p) { p.ctx.fillStyle = ink(); p.ctx.fillRect(((time / 20) % p.w), 8, 3, 3); } } }
    drawPointer();
    if (warpStart) drawWarp(time);
  }
  function tick(time) { frame = 0; if (document.hidden) return; if(document.documentElement.dataset.motion!==(reduced.matches?'still':'running')){motionChange();return;} if (time - previous > 33) { draw(time); previous = time; } if (!reduced.matches || warpStart) frame = requestAnimationFrame(tick); }
  function schedule() { if (document.hidden) return; if (reduced.matches && !warpStart) draw(); else if (!frame) frame = requestAnimationFrame(tick); }
  function resize() { entityDirty=true;drawBlueprint(); drawSpark(); draw(); schedule(); }
  function pointerMove(e) { if(pointer.x!==e.clientX/innerWidth||pointer.y!==e.clientY/innerHeight)pointer.movedAt=performance.now();pointer.x=e.clientX/innerWidth;pointer.y=e.clientY/innerHeight;pointer.visible=e.pointerType!=='touch';clearTimeout(pointerIdleTimer);pointerIdleTimer=setTimeout(schedule,720);schedule(); }
  function pointerLeave() { pointer.visible=false;schedule(); }
  function visibility() { introTime=null;diagramPrevious=null; if (document.hidden) { cancelAnimationFrame(frame); frame = 0; } else schedule(); }
  function updateDiagramControl(){diagramControl.disabled=reduced.matches;diagramControl.textContent=reduced.matches?'Diagram motion reduced':diagramPaused?'Play diagram':'Pause diagram';diagramControl.setAttribute('aria-pressed',String(!diagramPaused&&!reduced.matches));}
  function toggleDiagram(){diagramPaused=!diagramPaused;diagramPrevious=null;updateDiagramControl();schedule();}
  diagramControl.addEventListener('click',toggleDiagram);
  function motionChange() { document.documentElement.dataset.motion = reduced.matches ? 'still' : 'running';updateDiagramControl();diagramPrevious=null; if (reduced.matches) { cancelAnimationFrame(frame); frame = 0; if (warpResolve) finishWarp(true); } resize(); }
  const diagramObserver=new IntersectionObserver(entries=>{diagramVisible=entries[0].isIntersecting;diagramPrevious=null;schedule();});diagramObserver.observe(blueprint);
  const observer = new ResizeObserver(resize); [intro, blueprint, entity, spark,workspaceHands].forEach(c => observer.observe(c));
  window.addEventListener('pointermove', pointerMove, { passive: true });document.documentElement.addEventListener('pointerleave',pointerLeave);window.addEventListener('blur',pointerLeave); window.addEventListener('resize', resize); document.addEventListener('visibilitychange', visibility); reduced.addEventListener('change', motionChange); motionChange();
  return {
    setMode(value) { mode = value; if(value==='intro'){introProgress=0;handBeat=-1;handKey='';}introTime=null;resize(); },
    onTyping(text,offset,character,progress){typingTarget={text,offset};terminalCharacter=character;terminalProgress=progress;terminalTyping=true;schedule();},
    stopTyping(clear=false){terminalTyping=false;if(clear)typingTarget=null;schedule();},
    setSelection(index) { selected = index; blueprint.dataset.selection = String(selected); drawBlueprint(); },
    setLifecycle(value) {
      const next=['created','active','paused','suspended','terminated'].includes(value)?value:'unavailable';
      if(next!==savedLifecycle){savedLifecycle=next;lifecycleChangedAt=next==='unavailable'?-Infinity:performance.now();schedule();}
    },
    setWorm(value) { worm = value; schedule(); },
    onInput(value) { input = value; schedule(); },
    redraw: resize,
    warp() { if (reduced.matches) return Promise.resolve(true); if (warpResolve) return Promise.resolve(false); return new Promise(resolve => { warpResolve = resolve; warpStart = performance.now(); transition.dataset.progress='0';drawWarp(warpStart);transition.classList.add('active');schedule(); }); },
    cancelWarp() { finishWarp(false); },
    destroy() { cancelAnimationFrame(frame);clearTimeout(pointerIdleTimer); observer.disconnect();diagramObserver.disconnect();diagramControl.removeEventListener('click',toggleDiagram); window.removeEventListener('pointermove', pointerMove);document.documentElement.removeEventListener('pointerleave',pointerLeave);window.removeEventListener('blur',pointerLeave);tether.remove(); window.removeEventListener('resize', resize); document.removeEventListener('visibilitychange', visibility); reduced.removeEventListener('change', motionChange); finishWarp(false); },
  };
}
function format(value) { return Number(value).toLocaleString('en-US').replaceAll(',', "'"); }
