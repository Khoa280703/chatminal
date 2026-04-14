export const siteOrigin = "https://chatminal.com";

export function toAbsoluteUrl(pathname: string): string {
  return new URL(pathname, siteOrigin).toString();
}
