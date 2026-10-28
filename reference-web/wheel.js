/** Independently authored project wheel; no reference website source is vendored. */
export function createWheel({ host, canvas, items, onSelect, reducedMotion = false }) {
  if (!(host instanceof HTMLElement) || !(canvas instanceof HTMLCanvasElement)) {
    throw new TypeError("A wheel host and canvas are required.");
  }
  const entries = Array.isArray(items) ? items : [];
  const count = entries.length;
  const motion = typeof reducedMotion === "boolean"
    ? { matches: reducedMotion }
    : reducedMotion;
  const context = canvas.getContext("2d");
  const previousRole = host.getAttribute("role");
  host.setAttribute("role", "listbox");
  if (!host.hasAttribute("aria-label")) host.setAttribute("aria-label", "Supa Agent sections");
  host.style.touchAction = "pan-x pinch-zoom";
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.pointerEvents = "none";
  const list = document.createElement("ul");
  list.className = "wheel-list";
  list.setAttribute("role", "presentation");
  const buttons = entries.map((entry, index) => {
    const row = document.createElement("li");
    row.setAttribute("role", "presentation");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "wheel-item";
    button.dataset.index = String(index);
    button.setAttribute("role", "option");
    button.setAttribute("aria-label", entry.label);
    if (entry.thumb) {
      const thumbnail = document.createElement("img");
      thumbnail.className = "wheel-thumb";
      thumbnail.src = entry.thumb;
      thumbnail.alt = "";
      thumbnail.draggable = false;
      thumbnail.decoding = "async";
      thumbnail.referrerPolicy = "no-referrer";
      button.append(thumbnail);
    } else if(entry.preview) {
      const thumbnail=document.createElementNS('http://www.w3.org/2000/svg','svg');
      thumbnail.setAttribute('class','wheel-thumb wheel-preview');thumbnail.setAttribute('viewBox','0 0 200 80');thumbnail.setAttribute('aria-hidden','true');
      const use=document.createElementNS('http://www.w3.org/2000/svg','use');use.setAttribute('href',`/public/assets/navigation-previews.svg#${entry.preview}`);thumbnail.append(use);button.append(thumbnail);
    } else {
      const mark = document.createElement("span");
      mark.className = "wheel-thumb wheel-thumb-text";
      mark.setAttribute("aria-hidden", "true");
      mark.textContent = entry.mark ?? ">_";
      button.append(mark);
    }
    const label = document.createElement("span");
    label.className = "wheel-label";
    const name=document.createElement('span');name.className='wheel-name';name.textContent=entry.label.split(' · ')[0];label.append(name);
    if(entry.label.endsWith(' · planned')){const state=document.createElement('span');state.className='wheel-planned';state.textContent='planned';label.append(state);}
    button.append(label);
    row.append(button);
    list.append(row);
    return button;
  });
  host.append(list);

  let selected = 0;
  let position = 0;
  let target = 0;
  let width = 230;
  let height = 680;
  let frame = 0;
  let destroyed = false;
  let wheelDelta = 0;
  let lastWheel = 0;
  let pointer = null;
  let suppressClick = false;
  const modulo = (value) => count ? ((value % count) + count) % count : 0;
  const wrappedDistance = (value) => count
    ? ((value + count / 2) % count + count) % count - count / 2
    : 0;
  const random = (seed) => {
    const value = Math.sin(seed * 127.1 + 311.7) * 43758.5453123;
    return value - Math.floor(value);
  };
  const dimensions = () => {
    const cardWidth = Math.min(150, Math.max(104, width * 0.66));
    const cardHeight = cardWidth * 0.74;
    const gap = Math.min(88, Math.max(62, height / 7.5));
    return { cardWidth, cardHeight, gap };
  };
  const spine = (y) => {
    const normalized = (y - height / 2) / Math.max(height / 2, 1);
    return -16 + normalized * normalized * Math.min(90, width * 0.4)+Math.sin(y/Math.max(height,1)*13+position*.35)*4;
  };

  function drawBranch() {
    if (!context) return;
    const gutter = 110;
    const ink = getComputedStyle(host).color || "#ebe6db";
    context.clearRect(0, 0, width + gutter, height);
    context.save();
    context.translate(gutter + (matchMedia('(max-width:700px)').matches ? (width-dimensions().cardWidth)/2 : 0), 0);
    context.strokeStyle = ink;
    context.lineCap = "round";
    context.lineWidth = 1.45;
    context.globalAlpha = 0.72;
    context.beginPath();
    for (let y = 0; y <= height; y += 3) {
      const x = spine(y) + Math.sin(y * 0.012 + position * 0.5) * .6;
      if (y === 0) context.moveTo(x, y); else context.lineTo(x, y);
    }
    context.stroke();
    // Clustered fibers and junctions grow from the same spine; no idle redraws.
    for(let stem=0;stem<420;stem++){
      const seed=stem+Math.floor(position)*19,cluster=stem%5;
      const center=[.07,.29,.55,.76,.96][cluster]*height;
      const y=Math.max(0,Math.min(height,center+(random(seed+1)-.5)*height*.11));
      const x=spine(y),direction=random(seed+4)>.87?1:-1;
      const length=12+random(seed+3)**.65*(65+random(cluster+21)*45),tipX=x+direction*length,tipY=y-(random(seed+5)-.2)*90;
      context.globalAlpha=.18+random(seed+6)*.45;context.lineWidth=.45;
      context.beginPath();context.moveTo(x,y);context.bezierCurveTo(x+direction*length*.15,y-5,x+direction*length*.5,tipY+20,tipX,tipY);context.stroke();
      if(stem%3===0){context.fillStyle=ink;context.globalAlpha=.65;context.fillRect(tipX,tipY,1.7,1.7);}
    }
    const {gap,cardWidth}=dimensions();
    buttons.forEach((button,index)=>{
      const distance=wrappedDistance(index-position),y=height/2+Math.sin(distance*.17)/.17*gap;
      if(y<0||y>height)return;
      const cardX=3+Math.abs(distance)**2*Math.min(6,width/38);
      const scale=.64+Math.max(0,1-Math.abs(distance)*2)*.36,angle=-distance*.085;
      const endX=cardX+cardWidth/2-Math.cos(angle)*cardWidth*scale/2,endY=y-Math.sin(angle)*cardWidth*scale/2;
      context.globalAlpha=index === selected ? .6 : .22;context.lineWidth=.6;
      context.beginPath();context.moveTo(spine(y),y);context.bezierCurveTo(spine(y)+12,y-12,endX-8,endY-8,endX,endY);context.stroke();
    });
    context.restore();
  }

  function render() {
    const { cardWidth, cardHeight, gap } = dimensions();
    buttons.forEach((button, index) => {
      const distance = wrappedDistance(index - position);
      const y = height / 2 + Math.sin(distance * 0.17) / 0.17 * gap - cardHeight / 2;
      const x = 3 + (matchMedia('(max-width:700px)').matches ? (width-dimensions().cardWidth)/2 : 0) + Math.pow(Math.abs(distance), 2) * Math.min(6, width / 38);
      const scale = 0.64 + Math.max(0, 1 - Math.abs(distance) * 2) * 0.36;
      button.style.position = "absolute";
      button.style.left = "0";
      button.style.top = "0";
      button.style.width = `${cardWidth}px`;
      button.style.height = `${cardHeight}px`;
      button.style.transform = `translate(${x.toFixed(2)}px, ${y.toFixed(2)}px) rotate(${-distance * 0.085}rad) scale(${scale.toFixed(3)})`;
      button.style.setProperty("--shade", String(Math.min(0.72, Math.abs(distance) * 0.08 + (Math.abs(distance) > 0.2 ? 0.26 : 0))));
      button.style.zIndex = String(100 - Math.round(Math.abs(distance) * 10));
      button.style.visibility = y < -cardHeight || y > height ? "hidden" : "visible";
      button.setAttribute("aria-selected", String(index === selected));
      button.tabIndex = index === selected ? 0 : -1;
      button.classList.toggle("is-selected", index === selected);
    });
    drawBranch();
  }

  function animate() {
    frame = 0;
    if (destroyed || document.hidden) return;
    const difference = target - position;
    position = motion?.matches || Math.abs(difference) < 0.002 ? target : position + difference * 0.18;
    render();
    if (position !== target) frame = requestAnimationFrame(animate);
  }

  function schedule() {
    if (!frame && !destroyed && !document.hidden) frame = requestAnimationFrame(animate);
  }

  function select(index) {
    if (!count || !Number.isFinite(index) || destroyed) return;
    const next = modulo(Math.round(index));
    const changed = next !== selected;
    target += wrappedDistance(next - modulo(target));
    selected = next;
    // Keep accessibility state in sync before the visual interpolation finishes.
    buttons.forEach((button, itemIndex) => {
      button.setAttribute("aria-selected", String(itemIndex === selected));
      button.tabIndex = itemIndex === selected ? 0 : -1;
    });
    schedule();
    if (changed) onSelect?.(selected, entries[selected]);
  }

  function step(delta) {
    if (Number.isFinite(delta)) select(selected + Math.sign(delta));
  }

  function onClick(event) {
    if (suppressClick) {
      suppressClick = false;
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    const button = event.target.closest(".wheel-item");
    if (button && list.contains(button)) select(Number(button.dataset.index));
  }

  function onKey(event) {
    const directions = { ArrowDown: 1, j: 1, ArrowUp: -1, k: -1 };
    if (!(event.key in directions) && event.key !== "Home" && event.key !== "End") return;
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Home") select(0);
    else if (event.key === "End") select(count - 1);
    else step(directions[event.key]);
    buttons[selected]?.focus({ preventScroll: true });
  }

  function onWheel(event) {
    if (event.ctrlKey || !count) return;
    event.preventDefault();
    const now = performance.now();
    if (now - lastWheel > 180) wheelDelta = 0;
    lastWheel = now;
    wheelDelta += event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? height : 1);
    if (Math.abs(wheelDelta) >= 45) {
      step(wheelDelta);
      wheelDelta = 0;
    }
  }

  function onPointerDown(event) {
    if (!event.isPrimary || event.button !== 0) return;
    pointer = { id: event.pointerId, y: event.clientY, position, dragged: false };
    suppressClick = false;
  }

  function onPointerMove(event) {
    if (!pointer || pointer.id !== event.pointerId) return;
    const distance = pointer.y - event.clientY;
    if (!pointer.dragged && Math.abs(distance) < 7) return;
    if (!pointer.dragged) host.setPointerCapture(event.pointerId);
    pointer.dragged = true;
    event.preventDefault();
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    position = pointer.position + distance / dimensions().gap;
    render();
  }

  function onPointerEnd(event) {
    if (!pointer || pointer.id !== event.pointerId) return;
    const dragged = pointer.dragged;
    pointer = null;
    if (host.hasPointerCapture(event.pointerId)) host.releasePointerCapture(event.pointerId);
    if (dragged) {
      suppressClick = true;
      const next = modulo(Math.round(position));
      target = Math.round(position);
      const changed = selected !== next;
      selected = next;
      schedule();
      if (changed) onSelect?.(selected, entries[selected]);
    }
  }

  function resize() {
    const bounds = host.getBoundingClientRect();
    width = Math.max(1, bounds.width);
    height = Math.max(1, bounds.height);
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.ceil((width + 110) * ratio);
    canvas.height = Math.ceil(height * ratio);
    canvas.style.width = `${width + 110}px`;
    canvas.style.height = `${height}px`;
    canvas.style.left = "-110px";
    context?.setTransform(ratio, 0, 0, ratio, 0, 0);
    render();
  }

  function visibility() {
    if (document.hidden && frame) {
      cancelAnimationFrame(frame);
      frame = 0;
    } else if (!document.hidden) schedule();
  }

  const observer = new ResizeObserver(resize);
  observer.observe(host);
  host.addEventListener("click", onClick, true);
  host.addEventListener("keydown", onKey);
  host.addEventListener("wheel", onWheel, { passive: false });
  host.addEventListener("pointerdown", onPointerDown);
  host.addEventListener("pointermove", onPointerMove, { passive: false });
  host.addEventListener("pointerup", onPointerEnd);
  host.addEventListener("pointercancel", onPointerEnd);
  document.addEventListener("visibilitychange", visibility);
  motion?.addEventListener?.("change", schedule);
  resize();

  return {
    select,
    step,
    get index() { return selected; },
    destroy() {
      destroyed = true;
      if (frame) cancelAnimationFrame(frame);
      if (pointer && host.hasPointerCapture(pointer.id)) host.releasePointerCapture(pointer.id);
      pointer = null;
      observer.disconnect();
      host.removeEventListener("click", onClick, true);
      host.removeEventListener("keydown", onKey);
      host.removeEventListener("wheel", onWheel);
      host.removeEventListener("pointerdown", onPointerDown);
      host.removeEventListener("pointermove", onPointerMove);
      host.removeEventListener("pointerup", onPointerEnd);
      host.removeEventListener("pointercancel", onPointerEnd);
      document.removeEventListener("visibilitychange", visibility);
      motion?.removeEventListener?.("change", schedule);
      list.remove();
      if (previousRole === null) host.removeAttribute("role");
      else host.setAttribute("role", previousRole);
    },
  };
}
