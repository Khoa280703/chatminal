import { headers } from "next/headers";

import { detectLocaleFromAcceptLanguage, type Locale } from "@/lib/i18n";

export async function getPreferredLocale(): Promise<Locale> {
  const headerStore = await headers();
  return detectLocaleFromAcceptLanguage(headerStore.get("accept-language"));
}
