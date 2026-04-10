import Image from "next/image";
import Link from "next/link";

import { footerLinks } from "@/lib/landing-data";

export function SiteFooter() {
  return (
    <>
      <footer className="mt-24 flex w-full flex-col items-center justify-between gap-8 border-t border-white/15 bg-black px-8 py-12 md:flex-row">
        <div className="flex flex-col items-center gap-3 md:flex-row">
          <Link href="/" className="flex items-center gap-3">
            <Image
              src="/chatminal-logo.svg"
              alt="Chatminal logo"
              width={58}
              height={40}
              className="h-9 w-auto"
            />
            <span className="font-headline text-sm font-bold uppercase tracking-tight text-white">
              CHATMINAL
            </span>
          </Link>
          <span className="font-mono text-[10px] text-terminal-muted md:ml-3">
            © 2026 ALL RIGHTS RESERVED.
          </span>
        </div>
        <div className="flex flex-wrap items-center justify-center gap-6">
          {footerLinks.map((link) => (
            <Link
              key={link.label}
              href={link.href}
              className="font-mono text-[12px] uppercase tracking-widest text-[#d6d6d6] transition-colors hover:text-white"
            >
              {link.label}
            </Link>
          ))}
        </div>
      </footer>
    </>
  );
}
