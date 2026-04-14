import { notFound } from "next/navigation";

import { HtmlLangSync } from "@/components/html-lang-sync";
import { isSupportedLocale, supportedLocales } from "@/lib/i18n";

type LocaleLayoutProps = Readonly<{
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}>;

export function generateStaticParams() {
  return supportedLocales.map((locale) => ({ locale }));
}

export default async function LocaleLayout({ children, params }: LocaleLayoutProps) {
  const { locale } = await params;

  if (!isSupportedLocale(locale)) {
    notFound();
  }

  return (
    <>
      <HtmlLangSync locale={locale} />
      {children}
    </>
  );
}
