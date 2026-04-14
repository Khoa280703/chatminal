import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { DownloadGrid } from "@/components/download-grid";
import { FeaturesGrid } from "@/components/features-grid";
import { HeroSection } from "@/components/hero-section";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { isSupportedLocale, localeHrefLangs, supportedLocales, type Locale } from "@/lib/i18n";
import { getDictionary } from "@/lib/site-dictionary";

type LocalePageProps = {
  params: Promise<{ locale: string }>;
};

async function resolveLocale(params: Promise<{ locale: string }>): Promise<Locale> {
  const { locale } = await params;

  if (!isSupportedLocale(locale)) {
    notFound();
  }

  return locale;
}

export async function generateMetadata({ params }: LocalePageProps): Promise<Metadata> {
  const locale = await resolveLocale(params);
  const dictionary = getDictionary(locale);
  const languages = Object.fromEntries(
    supportedLocales.map((targetLocale) => [
      localeHrefLangs[targetLocale],
      `/${targetLocale}`,
    ]),
  );

  return {
    title: dictionary.meta.homeTitle,
    description: dictionary.meta.homeDescription,
    alternates: {
      canonical: `/${locale}`,
      languages: {
        ...languages,
        "x-default": "/en",
      },
    },
  };
}

export default async function LocalizedHomePage({ params }: LocalePageProps) {
  const locale = await resolveLocale(params);
  const dictionary = getDictionary(locale);

  return (
    <div id="top" className="terminal-shell min-h-screen bg-black">
      <div className="terminal-overlay" />
      <SiteHeader locale={locale} copy={dictionary.header} />
      <main className="relative overflow-x-hidden pb-20 pt-20">
        <HeroSection locale={locale} copy={dictionary.hero} previewCopy={dictionary.preview} />
        <FeaturesGrid items={dictionary.features.items} />
        <DownloadGrid copy={dictionary.downloads} />
      </main>
      <SiteFooter locale={locale} copy={dictionary.footer} />
    </div>
  );
}
