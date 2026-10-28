"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { PixelMark } from "./visuals";
import { PageMotion } from "./motion";

export const sections = [
  ["/agents", "01", "Agents"], ["/economy", "02", "Economy"], ["/memory", "03", "Memory"],
  ["/social", "04", "Social"], ["/trading", "05", "Trading"], ["/launch", "06", "Token launch"], ["/factory", "07", "Factory"],
] as const;

export function Shell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const closeMenu = useCallback(() => {
    if (!dialog.current?.open || closeTimer.current !== null) return;
    dialog.current.dataset.closing = "true";
    const duration = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 180;
    closeTimer.current = setTimeout(() => {
      dialog.current?.close();
      closeTimer.current = null;
    }, duration);
  }, []);

  useEffect(() => { closeMenu(); }, [pathname, closeMenu]);

  useEffect(() => {
    const desktop = window.matchMedia("(min-width: 641px)");
    const onResize = () => { if (desktop.matches) closeMenu(); };
    window.addEventListener("popstate", closeMenu);
    desktop.addEventListener("change", onResize);
    return () => {
      window.removeEventListener("popstate", closeMenu);
      desktop.removeEventListener("change", onResize);
      if (closeTimer.current !== null) clearTimeout(closeTimer.current);
    };
  }, [closeMenu]);

  useEffect(() => {
    if (!open) return;
    const body = document.body, root = document.documentElement;
    const scroll = window.scrollY;
    const previous = { position: body.style.position, top: body.style.top, width: body.style.width, overflow: body.style.overflow, rootOverflow: root.style.overflow };
    body.style.position = "fixed";
    body.style.top = `-${scroll}px`;
    body.style.width = "100%";
    body.style.overflow = "hidden";
    root.style.overflow = "hidden";
    return () => {
      body.style.position = previous.position;
      body.style.top = previous.top;
      body.style.width = previous.width;
      body.style.overflow = previous.overflow;
      root.style.overflow = previous.rootOverflow;
      window.scrollTo({ top: scroll, behavior: "instant" });
    };
  }, [open]);

  const links = [["/", "00", "Overview"], ...sections];
  const navigationLinks = () => links.map(([href, number, name]) => <Link key={href} href={href} aria-current={(href === "/" ? pathname === "/" : pathname === href || pathname.startsWith(`${href}/`)) ? "page" : undefined} onClick={closeMenu}><span>{number}</span> {name}</Link>);

  return <><a href="#content" className="skip-link">Skip to content</a>
    <header className="site-header">
      <div className="header-inner">
        <Link className="brand" href="/" aria-label="Supa Agent home"><PixelMark/><span>SUPA<span className="muted"> / AGENT</span></span></Link>
        <div className="header-right">
          <span className="environment">LOCAL / DEVNET</span>
          <a className="source-link" href="https://github.com/pyco404/SupaAgent" rel="noreferrer" target="_blank">SOURCE ↗</a>
          <button ref={trigger} className="menu-toggle" aria-expanded={open} aria-controls="mobile-navigation" aria-haspopup="dialog" onClick={() => {
            const modal = dialog.current;
            if (!modal || modal.open || !window.matchMedia("(max-width: 640px)").matches) return;
            delete modal.dataset.closing;
            modal.showModal();
            setOpen(true);
            closeButton.current?.focus({ preventScroll: true });
          }}>MENU +</button>
        </div>
      </div>
      <nav id="primary-navigation" className="navigation" aria-label="Main navigation"><div className="nav-inner">{navigationLinks()}</div></nav>
    </header>
    <dialog ref={dialog} id="mobile-navigation" className="mobile-navigation" aria-labelledby="mobile-navigation-title" onCancel={event => { event.preventDefault(); closeMenu(); }} onClose={() => {
      if (closeTimer.current !== null) clearTimeout(closeTimer.current);
      closeTimer.current = null;
      delete dialog.current?.dataset.closing;
      setOpen(false);
      if (window.matchMedia("(max-width: 640px)").matches) trigger.current?.focus({ preventScroll: true });
    }} onKeyDown={event => {
      if (event.key !== "Tab") return;
      const items = dialog.current?.querySelectorAll<HTMLElement>("a[href], button:not([disabled])");
      if (!items?.length) return;
      const first = items[0], last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    }}>
      <div className="mobile-navigation-header">
        <Link className="brand" href="/" onClick={closeMenu} aria-label="Supa Agent home"><PixelMark/><span>SUPA<span className="muted"> / AGENT</span></span></Link>
        <button ref={closeButton} className="mobile-navigation-close" aria-label="Close navigation" onClick={closeMenu}>×</button>
      </div>
      <div className="mobile-navigation-content">
        <h2 id="mobile-navigation-title" className="eyebrow">Mobile navigation</h2>
        <nav aria-label="Mobile navigation">{navigationLinks()}</nav>
        <p className="mobile-navigation-footer">LOCAL / DEVNET <span>READ-ONLY CONSOLE</span></p>
      </div>
    </dialog>
    <main id="content"><PageMotion path={pathname}>{children}</PageMotion></main><footer className="site-footer"><div><PixelMark/><span>SUPA AGENT</span><span className="muted">An experiment in autonomous survival.</span></div><p>READ-ONLY CONSOLE <span className="mint">/</span> SIMULATION + DEVNET TEST TOKENS</p><p className="muted">Signing, trading, publishing and agent creation are disabled.</p></footer></>;
}
