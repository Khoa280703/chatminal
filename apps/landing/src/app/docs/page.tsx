import type { Metadata } from "next";
import Link from "next/link";

import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { docsQuickLinks, docsSections } from "@/lib/user-docs-content";

export const metadata: Metadata = {
  title: "Chatminal Docs",
  description:
    "End-user documentation for installing, organizing, and using Chatminal.",
};

export default function DocsPage() {
  return (
    <div id="top" className="terminal-shell min-h-screen bg-black">
      <div className="terminal-overlay" />
      <SiteHeader />
      <main className="relative mx-auto flex w-full max-w-6xl flex-col gap-14 px-6 pb-24 pt-28 lg:flex-row lg:gap-16">
        <aside className="top-28 hidden h-fit lg:sticky lg:block lg:w-56 lg:shrink-0">
          <p className="font-mono text-[11px] uppercase tracking-[0.3em] text-terminal-muted">
            On this page
          </p>
          <div className="mt-8 border-l border-white/10 pl-4">
            {docsQuickLinks.map((item) => (
              <Link
                key={item.id}
                href={`#${item.id}`}
                className="block py-2 font-mono text-sm text-terminal-muted transition-colors hover:text-white"
              >
                {item.label}
              </Link>
            ))}
          </div>
        </aside>

        <div className="min-w-0 flex-1">
          <section className="max-w-4xl">
            <p className="font-mono text-[11px] uppercase tracking-[0.35em] text-terminal-muted">
              Chatminal user guide
            </p>
            <h2 className="mt-4 max-w-3xl font-headline text-4xl font-bold text-white md:text-6xl">
              Use Chatminal like a workspace you return to, not a terminal you throw away.
            </h2>
            <p className="mt-5 max-w-3xl font-mono text-base leading-7 text-terminal-muted">
              This page is for users, not contributors. It covers how to install Chatminal, how
              sessions and profiles fit together, how layouts behave, and what to expect when you
              come back to work later.
            </p>
          </section>

          <div className="mt-14 border-t border-white/10">
            {docsSections.map((section) => (
              <section
                key={section.id}
                id={section.id}
                className="scroll-mt-28 border-b border-white/10 py-10"
              >
                <p className="font-mono text-[11px] uppercase tracking-[0.35em] text-terminal-muted">
                  {section.label}
                </p>
                <h3 className="mt-3 max-w-3xl font-headline text-3xl font-bold text-white md:text-4xl">
                  {section.title}
                </h3>
                <p className="mt-4 max-w-3xl font-mono text-sm leading-7 text-terminal-muted">
                  {section.body}
                </p>

                <ul className="mt-6 space-y-3">
                  {section.bullets.map((bullet) => (
                    <li
                      key={bullet}
                      className="flex max-w-3xl items-start gap-3 font-mono text-sm leading-6 text-white/90"
                    >
                      <span
                        aria-hidden="true"
                        className="mt-3 h-px w-4 shrink-0 bg-white/35"
                      />
                      <span>{bullet}</span>
                    </li>
                  ))}
                </ul>

                {section.code && (
                  <pre className="mt-6 overflow-x-auto border-l border-white/20 pl-4 font-mono text-sm leading-6 text-white">
                    <code>{section.code}</code>
                  </pre>
                )}
              </section>
            ))}
          </div>

        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
