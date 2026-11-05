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

const description = "Supa Agency — One agent. One purpose. One token. A Solana-native agent platform in development, with persistent memory and bounded reasoning. SUPA is a planned token, not live.";

export const metadata: Metadata = {
  applicationName: "Supa Agency",
  title: { default: "Supa Agency — One agent. One purpose. One token.", template: "%s / Supa Agency" },
  description,
  openGraph: { type: "website", siteName: "Supa Agency", title: "Supa Agency — One agent. One purpose. One token.", description },
  twitter: { card: "summary", site: "@supa_agency", creator: "@supa_agency", title: "Supa Agency — One agent. One purpose. One token.", description },
  robots: { index: false, follow: false },
};
export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="en" className={VT323.variable}><body><Shell>{children}</Shell></body></html>;
}
