"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { PixelMark } from "./visuals";

export const sections = [
  ["/agents", "01", "Agents"], ["/economy", "02", "Economy"], ["/memory", "03", "Memory"],
  ["/social", "04", "Social"], ["/trading", "05", "Trading"], ["/launch", "06", "Token launch"], ["/factory", "07", "Factory"],
] as const;

export function Shell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);
  return <><a href="#content" className="skip-link">Skip to content</a><header className="site-header"><div className="header-inner"><Link className="brand" href="/" onClick={() => setOpen(false)} aria-label="Supa Agent home"><PixelMark/><span>SUPA<span className="muted"> / AGENT</span></span></Link><div className="header-right"><span className="environment"><span className="status-dot"/>LOCAL / DEVNET</span><a className="source-link" href="https://github.com/pyco404/SupaAgent" rel="noreferrer" target="_blank">SOURCE ↗</a><button className="menu-toggle" aria-expanded={open} aria-controls="primary-navigation" onClick={() => setOpen(!open)}>{open ? "CLOSE −" : "MENU +"}</button></div></div><nav id="primary-navigation" className={open ? "navigation open" : "navigation"} aria-label="Main navigation"><div className="nav-inner"><Link href="/" aria-current={pathname === "/" ? "page" : undefined} onClick={() => setOpen(false)}><span>00</span> Overview</Link>{sections.map(([href, number, name]) => <Link key={href} href={href} aria-current={pathname.startsWith(href) ? "page" : undefined} onClick={() => setOpen(false)}><span>{number}</span> {name}</Link>)}</div></nav></header><main id="content">{children}</main><footer className="site-footer"><div><PixelMark/><span>SUPA AGENT</span><span className="muted">An experiment in autonomous survival.</span></div><p>READ-ONLY CONSOLE <span className="mint">/</span> SIMULATION + DEVNET TEST TOKENS</p><p className="muted">Signing, trading, publishing and agent creation are disabled.</p></footer></>;
}
