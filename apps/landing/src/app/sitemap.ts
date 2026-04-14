import type { MetadataRoute } from "next";

import { localeHrefLangs, supportedLocales, withLocale } from "@/lib/i18n";
import { toAbsoluteUrl } from "@/lib/site-url";

function buildAlternates(pathname: string) {
  return {
    languages: Object.fromEntries(
      supportedLocales.map((locale) => [localeHrefLangs[locale], toAbsoluteUrl(withLocale(locale, pathname))]),
    ),
  };
}

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date();

  return supportedLocales.flatMap((locale) => [
    {
      url: toAbsoluteUrl(withLocale(locale, "/")),
      lastModified,
      changeFrequency: "weekly",
      priority: 1,
      alternates: buildAlternates("/"),
    },
    {
      url: toAbsoluteUrl(withLocale(locale, "/docs")),
      lastModified,
      changeFrequency: "monthly",
      priority: 0.7,
      alternates: buildAlternates("/docs"),
    },
  ]);
}
