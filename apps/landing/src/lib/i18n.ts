export const supportedLocales = ["en", "vi", "fr", "zh-cn", "ru", "hi"] as const;

export type Locale = (typeof supportedLocales)[number];

export const defaultLocale: Locale = "en";

export const localeLabels: Record<Locale, string> = {
  en: "English",
  vi: "Tiếng Việt",
  fr: "Français",
  "zh-cn": "简体中文",
  ru: "Русский",
  hi: "हिन्दी",
};

export const localeHrefLangs: Record<Locale, string> = {
  en: "en",
  vi: "vi",
  fr: "fr",
  "zh-cn": "zh-CN",
  ru: "ru",
  hi: "hi",
};

export function isSupportedLocale(value: string): value is Locale {
  return supportedLocales.includes(value as Locale);
}

export function stripLocaleFromPathname(pathname: string): string {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) {
    return "/";
  }
  if (isSupportedLocale(segments[0])) {
    const rest = segments.slice(1).join("/");
    return rest ? `/${rest}` : "/";
  }
  return pathname;
}

export function withLocale(locale: Locale, href: string): string {
  if (!href.startsWith("/") && !href.startsWith("#")) {
    return href;
  }

  if (href.startsWith("#")) {
    return `/${locale}${href}`;
  }

  const [path, hash = ""] = href.split("#");
  const localizedPath = stripLocaleFromPathname(path);
  const normalizedPath = localizedPath === "/" ? `/${locale}` : `/${locale}${localizedPath}`;

  return hash ? `${normalizedPath}#${hash}` : normalizedPath;
}

export function switchLocalePath(pathname: string, locale: Locale): string {
  return withLocale(locale, stripLocaleFromPathname(pathname));
}

export function detectLocaleFromAcceptLanguage(acceptLanguage: string | null): Locale {
  if (!acceptLanguage) {
    return defaultLocale;
  }

  const preferences = acceptLanguage
    .split(",")
    .map((entry) => {
      const [rawTag, rawQ] = entry.trim().split(";q=");
      const tag = rawTag.trim().toLowerCase();
      const q = rawQ ? Number.parseFloat(rawQ) : 1;
      return { tag, q: Number.isFinite(q) ? q : 1 };
    })
    .sort((a, b) => b.q - a.q);

  for (const preference of preferences) {
    if (preference.tag.startsWith("vi")) {
      return "vi";
    }
    if (preference.tag.startsWith("fr")) {
      return "fr";
    }
    if (preference.tag.startsWith("zh")) {
      return "zh-cn";
    }
    if (preference.tag.startsWith("ru")) {
      return "ru";
    }
    if (preference.tag.startsWith("hi")) {
      return "hi";
    }
    if (preference.tag.startsWith("en")) {
      return "en";
    }
  }

  return defaultLocale;
}
