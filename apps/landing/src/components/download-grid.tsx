import { LandingIcon } from "@/components/landing-icon";

import { downloadOptions } from "@/lib/landing-data";

export function DownloadGrid() {
  return (
    <section id="downloads" className="mx-auto mt-40 max-w-7xl px-6 text-center">
      <h2 className="mb-16 font-headline text-4xl font-bold uppercase tracking-tight text-white">
        Download
      </h2>
      <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
        {downloadOptions.map((option) => (
          <div
            key={option.label}
            className="group flex flex-col items-center border border-white/10 bg-white/5 p-10 transition-all hover:border-white/30"
          >
            <div className="mb-6 flex h-12 w-12 items-center justify-center border border-white/10 text-terminal-muted transition-colors group-hover:border-white/30 group-hover:text-white">
              <LandingIcon name={option.icon} className="h-7 w-7" />
            </div>
            <a
              href={option.href}
              download={option.directDownload}
              target={option.directDownload ? undefined : "_blank"}
              rel={option.directDownload ? undefined : "noreferrer"}
              className="w-full bg-white py-4 font-mono font-bold text-black transition-colors hover:bg-[#d4d4d4]"
            >
              {option.label}
            </a>
            <span className="mt-4 font-mono text-[10px] tracking-widest text-terminal-mutedSoft">
              {option.artifact}
            </span>
            {option.label.includes("MACOS") && (
              <span className="mt-2 font-mono text-[9px] text-terminal-mutedSoft">
                Choose Apple Silicon or Intel on GitHub Releases
              </span>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
