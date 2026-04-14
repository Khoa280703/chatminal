"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import {
  localeLabels,
  supportedLocales,
  switchLocalePath,
  stripLocaleFromPathname,
  withLocale,
  type Locale,
} from "@/lib/i18n";

type SiteHeaderProps = {
  locale: Locale;
  copy: {
    home: string;
    features: string;
    downloads: string;
    docs: string;
    downloadCta: string;
  };
};

export function SiteHeader({ locale, copy }: SiteHeaderProps) {
  const pathname = usePathname();
  const router = useRouter();
  const dropdownRef = useRef<HTMLDivElement | null>(null);
  const [isLanguageMenuOpen, setIsLanguageMenuOpen] = useState(false);
  const normalizedPathname = stripLocaleFromPathname(pathname);
  const navigationItems = [
    { label: copy.home, href: "/" },
    { label: copy.features, href: "/#features" },
    { label: copy.downloads, href: "/#downloads" },
    { label: copy.docs, href: "/docs" },
  ];

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (!dropdownRef.current?.contains(event.target as Node)) {
        setIsLanguageMenuOpen(false);
      }
    }

    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsLanguageMenuOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, []);

  function handleLanguageChange(targetLocale: Locale) {
    setIsLanguageMenuOpen(false);
    router.push(switchLocalePath(pathname, targetLocale));
  }

  return (
    <nav className="fixed inset-x-0 top-0 z-50 border-b border-white/5 bg-black/90 px-6 py-3 backdrop-blur-md">
      <div className="mx-auto flex w-full max-w-7xl items-center justify-between gap-4">
        <Link href={withLocale(locale, "/")} className="flex items-center gap-3">
          <Image
            src="/chatminal-logo.svg"
            alt="Chatminal logo"
            width={58}
            height={40}
            className="h-10 w-auto"
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
                ? normalizedPathname === "/docs"
                : item.href === "/"
                  ? normalizedPathname === "/"
                  : false;

            return (
              <Link
                key={item.label}
                href={withLocale(locale, item.href)}
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

        <div className="flex items-center gap-3">
          <div className="hidden items-center lg:flex">
            <div ref={dropdownRef} className="relative">
              <button
                type="button"
                aria-label="Language"
                aria-haspopup="menu"
                aria-expanded={isLanguageMenuOpen}
                onClick={() => setIsLanguageMenuOpen((current) => !current)}
                className="flex min-w-[72px] items-center justify-between gap-2 border border-white/10 bg-white/[0.04] px-3 py-2 font-mono text-[11px] uppercase tracking-[0.18em] text-white transition hover:border-white/25 hover:bg-white/[0.07]"
              >
                <span>{locale.toUpperCase()}</span>
                <svg
                  viewBox="0 0 10 6"
                  className={`h-[6px] w-[10px] text-terminal-mutedSoft transition-transform ${
                    isLanguageMenuOpen ? "rotate-180" : ""
                  }`}
                >
                  <path d="M1 1l4 4 4-4" stroke="currentColor" strokeWidth="1.2" fill="none" />
                </svg>
              </button>

              {isLanguageMenuOpen && (
                <div
                  role="menu"
                  aria-label="Language options"
                  className="absolute right-0 top-[calc(100%+8px)] min-w-[88px] overflow-hidden border border-white/10 bg-[#050505] p-1 shadow-[0_16px_40px_rgba(0,0,0,0.45)]"
                >
                  {supportedLocales.map((targetLocale) => {
                    const isActive = targetLocale === locale;

                    return (
                      <button
                        key={targetLocale}
                        type="button"
                        role="menuitemradio"
                        aria-checked={isActive}
                        title={localeLabels[targetLocale]}
                        onClick={() => handleLanguageChange(targetLocale)}
                        className={`flex w-full items-center justify-between px-3 py-2 font-mono text-[11px] uppercase tracking-[0.18em] transition ${
                          isActive
                            ? "bg-white text-black"
                            : "text-white hover:bg-white/[0.08]"
                        }`}
                      >
                        <span>{targetLocale.toUpperCase()}</span>
                        {isActive && <span className="text-[10px]">•</span>}
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </div>

          <Link
            href={withLocale(locale, "/#downloads")}
            className="bg-white px-6 py-2 font-mono text-sm font-bold tracking-tight text-black transition-colors hover:bg-[#d4d4d4]"
          >
            {copy.downloadCta}
          </Link>
        </div>
      </div>
    </nav>
  );
}
