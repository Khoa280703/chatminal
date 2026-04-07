import Link from "next/link";

import { footerLinks } from "@/lib/landing-data";

export function SiteFooter() {
  return (
    <>
      <footer className="mt-24 flex w-full flex-col items-center justify-between gap-8 border-t border-white/10 bg-black px-8 py-12 md:flex-row">
        <div className="flex flex-col items-center gap-2 md:flex-row">
          <span className="font-headline text-sm font-bold uppercase tracking-tight text-white">
            CHATMINAL_SYSTEMS
          </span>
          <span className="font-mono text-[10px] text-terminal-mutedSoft md:ml-4">
            © 2026 ALL RIGHTS RESERVED.
          </span>
        </div>
        <div className="flex flex-wrap items-center justify-center gap-6">
          {footerLinks.map((link) => (
            <Link
              key={link.label}
              href={link.href}
              className="font-mono text-[12px] uppercase tracking-widest text-terminal-muted transition-colors hover:text-white"
            >
              {link.label}
            </Link>
          ))}
        </div>
      </footer>
    </>
  );
}
