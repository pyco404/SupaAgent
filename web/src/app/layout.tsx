import type { Metadata } from "next";
import "@fontsource/vt323/latin-400.css";
import "./globals.css";
import { Shell } from "@/components/shell";

export const metadata: Metadata = {
  title: { default: "Supa Agent — Born to think. Built to survive.", template: "%s / Supa Agent" },
  description: "A read-only window into autonomous intelligence, persistent memory and a simulated economy. Solana Devnet only.",
  robots: { index: false, follow: false },
};
export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="en"><body><Shell>{children}</Shell></body></html>;
}
