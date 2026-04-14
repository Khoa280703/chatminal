import { TerminalWindowPreview } from "@/components/terminal-window-preview";
import type { Locale } from "@/lib/i18n";
import type { SiteDictionary } from "@/lib/site-dictionary";

type HeroSectionProps = {
  locale: Locale;
  copy: SiteDictionary["hero"];
  previewCopy: SiteDictionary["preview"];
};

export function HeroSection({ locale, copy, previewCopy }: HeroSectionProps) {
  return (
    <section className="mx-auto mt-12 flex max-w-7xl flex-col items-center px-6 text-center">
      <h1 className="text-balance mb-8 max-w-4xl font-headline text-5xl font-bold leading-none tracking-tight text-white md:text-8xl">
        {copy.title}
      </h1>
      <p className="mb-10 max-w-2xl font-mono text-lg text-terminal-muted">
        {copy.description}
      </p>
      <TerminalWindowPreview locale={locale} copy={previewCopy} />
    </section>
  );
}
