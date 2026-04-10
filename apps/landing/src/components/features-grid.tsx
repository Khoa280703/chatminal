import { LandingIcon } from "@/components/landing-icon";
import { featureItems } from "@/lib/landing-data";

export function FeaturesGrid() {
  return (
    <section id="features" className="mx-auto mt-16 max-w-7xl px-6 md:mt-32">
      <div className="grid grid-cols-1 gap-8 md:grid-cols-3">
        {featureItems.map((feature) => (
          <article
            key={feature.title}
            className="group border border-white/10 bg-black p-8 transition-all duration-300 hover:border-white"
          >
            <div className="mb-6 flex h-12 w-12 items-center justify-center border border-white/10 bg-white/5 text-white">
              <LandingIcon name={feature.icon} className="h-6 w-6" />
            </div>
            <h2 className="mb-4 font-headline text-2xl font-bold uppercase text-white">
              {feature.title}
            </h2>
            <p className="font-mono text-sm leading-relaxed text-terminal-muted">
              {feature.description}
            </p>
          </article>
        ))}
      </div>
    </section>
  );
}
