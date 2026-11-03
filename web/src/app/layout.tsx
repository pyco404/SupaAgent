import type { Metadata } from "next";
import localFont from "next/font/local";
import "./globals.css";
import { Shell } from "@/components/shell";

const VT323 = localFont({
  src: "../../node_modules/@fontsource/vt323/files/vt323-latin-400-normal.woff2",
  variable: "--font-supa-pixel",
  display: "swap",
  weight: "400",
  fallback: ["monospace"],
  preload: true,
});

const description = "Supa Agent — Born to think. Built to survive. A Solana-native agent platform in development, with persistent memory and bounded reasoning. SUPA is a planned token, not live.";

export const metadata: Metadata = {
  metadataBase: new URL("https://trysupaagent.xyz"),
  applicationName: "Supa Agent",
  title: { default: "Supa Agent — Born to think. Built to survive.", template: "%s / Supa Agent" },
  description,
  openGraph: { type: "website", siteName: "Supa Agent", title: "Supa Agent — Born to think. Built to survive.", description, url: "https://trysupaagent.xyz" },
  twitter: { card: "summary", site: "@trysupaagent", creator: "@trysupaagent", title: "Supa Agent — Born to think. Built to survive.", description },
  robots: { index: false, follow: false },
};
export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="en" className={VT323.variable}><body><Shell>{children}</Shell></body></html>;
}
