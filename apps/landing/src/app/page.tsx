import { DownloadGrid } from "@/components/download-grid";
import { FeaturesGrid } from "@/components/features-grid";
import { HeroSection } from "@/components/hero-section";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";

export default function HomePage() {
  return (
    <div id="top" className="terminal-shell min-h-screen bg-black">
      <div className="terminal-overlay" />
      <SiteHeader />
      <main className="relative overflow-x-hidden pb-20 pt-20">
        <HeroSection />
        <FeaturesGrid />
        <DownloadGrid />
      </main>
      <SiteFooter />
    </div>
  );
}
