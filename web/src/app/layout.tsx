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

export const metadata: Metadata = {
  title: { default: "Supa Agent — Born to think. Built to survive.", template: "%s / Supa Agent" },
  description: "A read-only window into autonomous intelligence, persistent memory and a simulated economy. Solana Devnet only.",
  robots: { index: false, follow: false },
};
export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="en" className={VT323.variable}><body><Shell>{children}</Shell></body></html>;
}
