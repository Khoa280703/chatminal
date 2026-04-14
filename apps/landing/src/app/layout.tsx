import type { Metadata } from "next";
import "@xterm/xterm/css/xterm.css";
import { JetBrains_Mono, Space_Grotesk } from "next/font/google";

import { siteOrigin } from "@/lib/site-url";

import "./globals.css";

const spaceGrotesk = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-space-grotesk",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
});

export const metadata: Metadata = {
  metadataBase: new URL(siteOrigin),
  title: "CHATMINAL | Vibe Coding Environment",
  description:
    "A monochrome landing page for Chatminal, adapted from the Stitch design project.",
  icons: {
    icon: "/chatminal-logo.svg",
    shortcut: "/chatminal-logo.svg",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${spaceGrotesk.variable} ${jetbrainsMono.variable}`}
    >
      <body className="font-body text-terminal-text antialiased">{children}</body>
    </html>
  );
}
