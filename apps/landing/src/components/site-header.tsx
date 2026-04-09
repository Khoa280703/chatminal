"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { navigationItems } from "@/lib/landing-data";

export function SiteHeader() {
  const pathname = usePathname();

  return (
    <nav className="fixed inset-x-0 top-0 z-50 border-b border-white/5 bg-black/90 px-6 py-3 backdrop-blur-md">
      <div className="mx-auto flex w-full max-w-7xl items-center justify-between gap-4">
        <Link href="/" className="flex items-center gap-3">
          <Image
            src="/chatminal-logo.png"
            alt="Chatminal logo"
            width={32}
            height={32}
            className="h-8 w-8"
            priority
          />
          <span className="font-headline text-xl font-bold uppercase tracking-tight text-white">
            CHATMINAL
          </span>
        </Link>

        <div className="hidden items-center gap-8 font-mono text-[13px] tracking-widest md:flex">
          {navigationItems.map((item) => {
            const isActive =
              item.href === "/docs"
                ? pathname === "/docs"
                : item.href === "/"
                  ? pathname === "/"
                  : false;

            return (
              <Link
                key={item.label}
                href={item.href}
                className={
                  isActive
                    ? "border-b border-white pb-1 text-white"
                    : "text-terminal-muted transition-colors hover:text-white"
                }
              >
                {item.label}
              </Link>
            );
          })}
        </div>

        <Link
          href="/#downloads"
          className="bg-white px-6 py-2 font-mono text-sm font-bold tracking-tight text-black transition-colors hover:bg-[#d4d4d4]"
        >
          DOWNLOAD
        </Link>
      </div>
    </nav>
  );
}
