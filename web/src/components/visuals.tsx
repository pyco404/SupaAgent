"use client";

import { useState } from "react";
import type { Ledger } from "@/lib/contracts";
import { units } from "@/lib/contracts";

export function PixelMark({ large = false }: { large?: boolean }) {
  return <svg className={large ? "pixel-mark large" : "pixel-mark"} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" shapeRendering="crispEdges"><path d="M7 2h10v2h4v4h2v10h-4v4H5v-4H1V8h2V4h4z"/><path d="M6 9h4v5H6zm8 0h4v5h-4zM8 17h8v2H8z" fill="var(--bg)"/></svg>;
}

const stages = [
  { name: "Observe", detail: "Read persisted events and retrieve relevant memories. A hypothesis stays a hypothesis.", symbol: "01" },
  { name: "Reason", detail: "Propose a bounded plan against current goals. Personality shapes the voice, never the permissions.", symbol: "02" },
  { name: "Authorize", detail: "Application code checks capabilities, lifecycle, tool limits and available simulated funds.", symbol: "03" },
  { name: "Record", detail: "Persist the result, costs and lessons. Rejected actions are recorded too. Publishing and signing stay disabled.", symbol: "04" },
];
export function SurvivalDiagram() {
  const [selected, setSelected] = useState(0);
  const [selectionCount, setSelectionCount] = useState(0);
  return <figure className="diagram panel" data-diagram-mode="explanation" aria-label="Interactive reasoning architecture">
    <div className="figure-bar"><span>FIG. 01 / THE SURVIVAL LOOP</span><span className="mint">SYSTEM DESIGN</span></div>
    <div className="diagram-core">
      <svg className="connections" viewBox="0 0 480 280" fill="none" aria-hidden="true">
        {[{ name: "EVENTS", x: 376, y: 220, path: "M340 220H300V168H270" }, { name: "GOALS", x: 76, y: 64, path: "M112 64H180V112H210" }, { name: "POLICY", x: 76, y: 220, path: "M112 220H180V168H210" }, { name: "MEMORY", x: 376, y: 64, path: "M340 64H300V112H270" }].map((node, i) => <g key={node.name} data-selected={selected === i}>
          <path className="connection-rail" d={node.path}/>
          {selected === i && <path key={selectionCount} className={selectionCount ? "connection-selected connection-trace" : "connection-selected"} d={node.path}/>}
          <rect x={node.x - 42} y={node.y - 19} width="84" height="38" className="connection-node"/>
          <text x={node.x} y={node.y + 5} textAnchor="middle" className="connection-text">{node.name}</text>
        </g>)}
      </svg>
      <div className="core-frame"><PixelMark large/><span>SUPA / CORE</span></div>
      <span className="diagram-mode">SELECTED STAGE / EXPLANATION ONLY</span>
    </div>
    <div className="stage-grid">{stages.map((stage, i) => <button key={stage.name} aria-pressed={selected === i} onClick={() => { if (selected !== i) { setSelected(i); setSelectionCount(count => count + 1); } }} className={selected === i ? "stage selected" : "stage"}><span>{stage.symbol}</span>{stage.name}</button>)}</div>
    <figcaption aria-live="polite"><div className="diagram-explanation" key={selected}><span className="mint">{stages[selected].symbol} / {stages[selected].name.toUpperCase()}</span><p>{stages[selected].detail}</p><small>Interactive explanation · does not run an agent</small></div></figcaption>
  </figure>;
}

export function ExpenseChart({ entries }: { entries: Ledger[] }) {
  const [view, setView] = useState<"chart" | "table">("chart");
  const ordered = [...entries].reverse();
  const expenses = ordered.filter(e => e.amount_microusd < 0);
  const max = Math.max(1, ...expenses.map(e => Math.abs(e.amount_microusd)));
  return <section className="panel chart-panel"><div className="panel-heading"><h2>Cost of staying curious</h2><div className="segmented" aria-label="Expense view">{(["chart", "table"] as const).map(v => <button key={v} onClick={() => setView(v)} aria-pressed={view === v}>{v}</button>)}</div></div>
    <p className="muted">Simulated expenses only · latest {entries.length} ledger entries · no projected returns</p>
    {expenses.length === 0 ? <div className="empty"><span>∅</span><h3>No recorded expenses</h3><p>The chart will appear when persisted simulated expense entries are available.</p></div> : view === "chart" ? <figure><svg className="expense-chart" viewBox="0 0 720 200" role="img" aria-label={`${expenses.length} recorded simulated expense entries`} preserveAspectRatio="none"><path d="M0 1H720M0 65H720M0 130H720M0 199H720" stroke="var(--border)" fill="none"/>{expenses.map((e, i) => { const width = 700 / expenses.length; const height = Math.max(2, Math.abs(e.amount_microusd) / max * 175); return <rect className="expense-bar" key={e.id} x={10 + i * width} y={195 - height} width={Math.max(1, width - 4)} height={height} fill="var(--mint)"><title>{e.kind}: simulated ${units(Math.abs(e.amount_microusd))}</title></rect>; })}</svg><figcaption className="chart-caption"><span>OLDEST RECORDED → NEWEST</span><span>MAX SIM. ${units(max)}</span></figcaption></figure> : <div className="table-scroll"><table><caption className="sr-only">Simulated expense ledger</caption><thead><tr><th>Recorded (UTC)</th><th>Kind</th><th>Simulated USD</th></tr></thead><tbody>{expenses.map(e => <tr key={e.id}><td>{new Date(e.created_at).toISOString().slice(0, 19).replace("T", " ")}</td><td>{e.kind}</td><td>{units(e.amount_microusd)}</td></tr>)}</tbody></table></div>}
  </section>;
}
