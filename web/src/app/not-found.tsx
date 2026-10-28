import Link from "next/link";

export default function NotFound() {
  return <section className="page-width not-found"><p className="eyebrow mint">404 / OUTSIDE THE MAP</p><h1>No signal here.</h1><p>This page or agent identifier does not exist.</p><Link className="button primary" href="/">BACK TO OVERVIEW ↗</Link></section>;
}
