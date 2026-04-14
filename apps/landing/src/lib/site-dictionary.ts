import type { Locale } from "@/lib/i18n";
import { defaultLocale } from "@/lib/i18n";

import { enDictionary } from "@/lib/locales/en";
import { frDictionary } from "@/lib/locales/fr";
import { hiDictionary } from "@/lib/locales/hi";
import { ruDictionary } from "@/lib/locales/ru";
import { viDictionary } from "@/lib/locales/vi";
import { zhCnDictionary } from "@/lib/locales/zh-cn";

export type DocsMethod = {
  id: string;
  label: string;
  title: string;
  body: string;
  code: string;
};

export type DocsSection = {
  id: string;
  label: string;
  title: string;
  body: string;
  bullets: string[];
  code?: string;
  methods?: DocsMethod[];
};

export type DownloadMethod = {
  id: string;
  label: string;
  description: string;
  code: string;
};

export type DownloadPlatform = {
  id: "macos" | "linux" | "windows";
  label: string;
  icon: string;
  artifact: string;
  downloadHref: string;
  directDownload: boolean;
  downloadLabel: string;
  helperText: string;
  methods: DownloadMethod[];
};

export type FeatureItem = {
  icon: string;
  title: string;
  description: string;
};

export type SiteDictionary = {
  meta: {
    homeTitle: string;
    homeDescription: string;
    docsTitle: string;
    docsDescription: string;
  };
  header: {
    home: string;
    features: string;
    downloads: string;
    docs: string;
    downloadCta: string;
    languageLabel: string;
  };
  hero: {
    title: string;
    description: string;
  };
  features: {
    items: FeatureItem[];
  };
  downloads: {
    title: string;
    description: string;
    copiedLabel: string;
    copyAndRunLabel: string;
    terminalLabel: string;
    platforms: DownloadPlatform[];
  };
  footer: {
    copyright: string;
    home: string;
    userDocs: string;
    githubRepo: string;
    statusLog: string;
    devDocs: string;
  };
  docs: {
    sidebarTitle: string;
    eyebrow: string;
    title: string;
    description: string;
    sections: DocsSection[];
  };
  preview: {
    welcomeBack: string;
    tipsTitle: string;
    tipsBody: string;
    recentTitle: string;
    recentEmpty: string;
    geminiWaiting: string;
  };
};

const dictionaries: Record<Locale, SiteDictionary> = {
  en: enDictionary,
  vi: viDictionary,
  fr: frDictionary,
  "zh-cn": zhCnDictionary,
  ru: ruDictionary,
  hi: hiDictionary,
};

export function getDictionary(locale: Locale): SiteDictionary {
  return dictionaries[locale] ?? dictionaries[defaultLocale];
}
