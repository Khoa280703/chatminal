import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { isSupportedLocale, localeHrefLangs, supportedLocales, type Locale } from "@/lib/i18n";
import { getDictionary } from "@/lib/site-dictionary";

type LocaleDocsPageProps = {
  params: Promise<{ locale: string }>;
};

async function resolveLocale(params: Promise<{ locale: string }>): Promise<Locale> {
  const { locale } = await params;

  if (!isSupportedLocale(locale)) {
    notFound();
  }

  return locale;
}

export async function generateMetadata({ params }: LocaleDocsPageProps): Promise<Metadata> {
  const locale = await resolveLocale(params);
  const dictionary = getDictionary(locale);
  const languages = Object.fromEntries(
    supportedLocales.map((targetLocale) => [
      localeHrefLangs[targetLocale],
      `/${targetLocale}/docs`,
    ]),
  );

  return {
    title: dictionary.meta.docsTitle,
    description: dictionary.meta.docsDescription,
    alternates: {
      canonical: `/${locale}/docs`,
      languages: {
        ...languages,
        "x-default": "/en/docs",
      },
    },
  };
}

export default async function LocalizedDocsPage({ params }: LocaleDocsPageProps) {
  const locale = await resolveLocale(params);
  const dictionary = getDictionary(locale);

  return (
    <div id="top" className="terminal-shell min-h-screen bg-black">
      <div className="terminal-overlay" />
      <SiteHeader locale={locale} copy={dictionary.header} />
      <main className="relative mx-auto flex w-full max-w-6xl flex-col gap-14 px-6 pb-24 pt-28 lg:flex-row lg:gap-16">
        <aside className="top-28 hidden h-fit lg:sticky lg:block lg:w-56 lg:shrink-0">
          <p className="font-mono text-[11px] uppercase tracking-[0.3em] text-terminal-muted">
            {dictionary.docs.sidebarTitle}
          </p>
          <div className="mt-8 border-l border-white/10 pl-4">
            {dictionary.docs.sections.map((item) => (
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
              {dictionary.docs.eyebrow}
            </p>
            <h2 className="mt-4 max-w-3xl font-headline text-4xl font-bold text-white md:text-6xl">
              {dictionary.docs.title}
            </h2>
            <p className="mt-5 max-w-3xl font-mono text-base leading-7 text-terminal-muted">
              {dictionary.docs.description}
            </p>
          </section>

          <div className="mt-14 border-t border-white/10">
            {dictionary.docs.sections.map((section) => (
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

                {section.methods && (
                  <div className="mt-8 space-y-6">
                    {section.methods.map((method) => (
                      <div key={method.id} className="border-l border-white/15 pl-4">
                        <p className="font-mono text-[11px] uppercase tracking-[0.3em] text-terminal-muted">
                          {method.label}
                        </p>
                        <h4 className="mt-3 font-headline text-2xl font-bold text-white">
                          {method.title}
                        </h4>
                        <p className="mt-3 max-w-3xl font-mono text-sm leading-6 text-terminal-muted">
                          {method.body}
                        </p>
                        <pre className="mt-5 overflow-x-auto border-l border-white/20 pl-4 font-mono text-sm leading-6 text-white">
                          <code>{method.code}</code>
                        </pre>
                      </div>
                    ))}
                  </div>
                )}
              </section>
            ))}
          </div>
        </div>
      </main>
      <SiteFooter locale={locale} copy={dictionary.footer} />
    </div>
  );
}
