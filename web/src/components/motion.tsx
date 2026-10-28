"use client";

import { useEffect, useRef, type ReactNode } from "react";

/** Animate the final, exact value; never interpolate or round financial data. */
export function ExactValue({ children, label }: { children: string; label?: string }) {
  const element = useRef<HTMLSpanElement>(null);
  const previous = useRef(children);
  useEffect(() => {
    const changed = previous.current !== children;
    previous.current = children;
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (!changed || preference.matches) return;
    const animation = element.current?.animate([
      { opacity: 0.65, transform: "translateY(3px)" },
      { opacity: 1, transform: "translateY(0)" },
    ], { duration: 260, easing: "cubic-bezier(.2,.7,.2,1)" });
    const onPreference = () => { if (preference.matches) animation?.cancel(); };
    preference.addEventListener("change", onPreference);
    return () => { animation?.cancel(); preference.removeEventListener("change", onPreference); };
  }, [children]);
  return <span ref={element} className="value exact-value" aria-label={label ? `${label}: ${children}` : undefined} aria-live="polite" aria-atomic="true">{children}</span>;
}

/** Progressive enhancement: server-rendered content stays visible without JS. */
export function PageMotion({ children, path }: { children: ReactNode; path: string }) {
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (!("IntersectionObserver" in window)) return;
    const elements = root.current?.querySelectorAll<HTMLElement>(".editorial-section, .principle-grid article, .horizon") ?? [];
    const reveal = (element: HTMLElement) => { element.dataset.reveal = "visible"; };
    const observer = new IntersectionObserver(entries => {
      for (const entry of entries) if (entry.isIntersecting) {
        reveal(entry.target as HTMLElement); observer.unobserve(entry.target);
      }
    }, { threshold: 0.08, rootMargin: "0px 0px -24px 0px" });
    for (const element of elements) {
      if (!preference.matches && element.getBoundingClientRect().top >= window.innerHeight) {
        element.dataset.reveal = "pending"; observer.observe(element);
      }
    }
    const onPreference = () => {
      if (preference.matches) { elements.forEach(reveal); observer.disconnect(); }
    };
    preference.addEventListener("change", onPreference);
    return () => {
      observer.disconnect(); preference.removeEventListener("change", onPreference);
      elements.forEach(element => { delete element.dataset.reveal; });
    };
  }, [path]);
  return <div ref={root} className="page-motion" key={path}>{children}</div>;
}
