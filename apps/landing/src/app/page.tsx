import { redirect } from "next/navigation";

import { withLocale } from "@/lib/i18n";
import { getPreferredLocale } from "@/lib/i18n-server";

export default async function HomePage() {
  const locale = await getPreferredLocale();

  redirect(withLocale(locale, "/"));
}
