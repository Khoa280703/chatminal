"use client";

import { useMemo, useState } from "react";

import { LandingIcon } from "@/components/landing-icon";
import type { SiteDictionary } from "@/lib/site-dictionary";

type TerminalTokenTone = "prompt" | "command" | "flag" | "url" | "string" | "plain";

function classifyToken(token: string, index: number) {
  const value = token.trim();

  if (!value) {
    return "plain" as TerminalTokenTone;
  }
  if (index === 0) {
    return "command" as TerminalTokenTone;
  }
  if (value.startsWith("-")) {
    return "flag" as TerminalTokenTone;
  }
  if (
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.includes("github.com") ||
    value.includes("chatminal.com")
  ) {
    return "url" as TerminalTokenTone;
  }
  if (value.startsWith('"') || value.startsWith("'")) {
    return "string" as TerminalTokenTone;
  }
  return "plain" as TerminalTokenTone;
}

function tokenToneClassName(tone: TerminalTokenTone) {
  switch (tone) {
    case "prompt":
      return "text-[#28c840]";
    case "command":
      return "text-[#7dd3fc]";
    case "flag":
      return "text-[#c4b5fd]";
    case "url":
      return "text-[#facc15]";
    case "string":
      return "text-[#fda4af]";
    default:
      return "text-white";
  }
}

function renderTerminalCommand(code: string, platformId: string) {
  const prompt = platformId === "windows" ? "PS>" : "$";

  return code.split("\n").map((line, lineIndex) => {
    const parts = line.split(/(\s+)/);
    return (
      <div key={`${platformId}-${lineIndex}`} className="flex flex-wrap items-start">
        <span className="mr-3 select-none font-semibold text-[#28c840]">{prompt}</span>
        <span className="flex-1">
          {parts.map((part, tokenIndex) => {
            const tone = classifyToken(part, tokenIndex);
            return (
              <span
                key={`${platformId}-${lineIndex}-${tokenIndex}`}
                className={tokenToneClassName(tone)}
              >
                {part}
              </span>
            );
          })}
        </span>
      </div>
    );
  });
}

type DownloadGridProps = {
  copy: SiteDictionary["downloads"];
};

export function DownloadGrid({ copy }: DownloadGridProps) {
  const downloadPlatforms = copy.platforms;
  const [platformId, setPlatformId] = useState(downloadPlatforms[0]?.id ?? "macos");
  const [methodIdByPlatform, setMethodIdByPlatform] = useState<Record<string, string>>({
    macos: "brew",
    linux: "bash",
    windows: "powershell",
  });
  const [copiedMethodId, setCopiedMethodId] = useState<string | null>(null);

  const activePlatform = useMemo(
    () => downloadPlatforms.find((platform) => platform.id === platformId) ?? downloadPlatforms[0],
    [downloadPlatforms, platformId]
  );
  const activeMethod =
    activePlatform.methods.find(
      (method) => method.id === methodIdByPlatform[activePlatform.id]
    ) ?? activePlatform.methods[0];

  async function handleCopyCommand() {
    try {
      await navigator.clipboard.writeText(activeMethod.code);
      setCopiedMethodId(`${activePlatform.id}:${activeMethod.id}`);
      window.setTimeout(() => {
        setCopiedMethodId((current) =>
          current === `${activePlatform.id}:${activeMethod.id}` ? null : current
        );
      }, 1600);
    } catch {
      setCopiedMethodId(null);
    }
  }

  const isCopied = copiedMethodId === `${activePlatform.id}:${activeMethod.id}`;

  return (
    <section id="downloads" className="mx-auto mt-40 max-w-7xl px-6">
      <div className="mb-10 text-center">
        <h2 className="font-headline text-4xl font-bold uppercase tracking-tight text-white">
          {copy.title}
        </h2>
        <p className="mx-auto mt-4 max-w-2xl font-mono text-sm text-terminal-mutedSoft">
          {copy.description}
        </p>
      </div>

      <div className="overflow-hidden border border-white/10 bg-[#050505] shadow-[0_32px_100px_rgba(0,0,0,0.45)]">
        <div className="flex flex-col gap-4 border-b border-white/10 bg-[#0a0a0a] px-4 py-4 md:flex-row md:items-center md:justify-between">
          <div className="flex items-center gap-2">
            <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
            <span className="h-3 w-3 rounded-full bg-[#ffbd2f]" />
            <span className="h-3 w-3 rounded-full bg-[#28c840]" />
            <span className="ml-3 font-mono text-[11px] uppercase tracking-[0.34em] text-terminal-mutedSoft">
              {copy.terminalLabel}
            </span>
          </div>

          <div className="flex flex-wrap gap-2">
            {downloadPlatforms.map((platform) => {
              const isActive = platform.id === activePlatform.id;
              return (
                <button
                  key={platform.id}
                  type="button"
                  onClick={() => setPlatformId(platform.id)}
                  className={`inline-flex items-center gap-2 border px-3 py-2 font-mono text-[11px] uppercase tracking-[0.24em] transition ${
                    isActive
                      ? "border-white bg-white text-black"
                      : "border-white/10 bg-white/[0.03] text-terminal-mutedSoft hover:border-white/30 hover:text-white"
                  }`}
                >
                  <LandingIcon name={platform.icon} className="h-4 w-4" />
                  {platform.label}
                </button>
              );
            })}
          </div>
        </div>

        <div className="grid gap-0 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="border-b border-white/10 p-4 lg:border-b-0 lg:border-r lg:p-6">
            <div className="bg-black/70 p-4 md:p-5">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-2">
                  {activePlatform.methods.map((method) => {
                    const isActive = method.id === activeMethod.id;
                    return (
                      <button
                        key={method.id}
                        type="button"
                        onClick={() =>
                          setMethodIdByPlatform((current) => ({
                            ...current,
                            [activePlatform.id]: method.id,
                          }))
                        }
                        className={`px-2.5 py-1.5 font-mono text-[11px] uppercase tracking-[0.24em] transition ${
                          isActive
                            ? "bg-white text-black"
                            : "bg-white/[0.04] text-terminal-mutedSoft hover:bg-white/[0.08] hover:text-white"
                        }`}
                      >
                        {method.label}
                      </button>
                    );
                  })}
                </div>
                <button
                  type="button"
                  onClick={handleCopyCommand}
                  className="bg-white/[0.06] px-3 py-2 font-mono text-[10px] uppercase tracking-[0.24em] text-terminal-muted transition hover:bg-white/[0.12] hover:text-white"
                >
                  {isCopied ? copy.copiedLabel : copy.copyAndRunLabel}
                </button>
              </div>
              <p className="mb-4 max-w-2xl font-mono text-xs leading-6 text-terminal-mutedSoft">
                {activeMethod.description}
              </p>
              <pre className="overflow-x-auto whitespace-pre-wrap break-all font-mono text-[13px] leading-7">
                <code>{renderTerminalCommand(activeMethod.code, activePlatform.id)}</code>
              </pre>
            </div>
          </div>

          <div className="p-4 lg:p-6">
            <div className="flex flex-col items-center text-center">
              <div className="flex h-11 w-11 items-center justify-center bg-white/[0.04] text-terminal-muted">
                <LandingIcon name={activePlatform.icon} className="h-6 w-6" />
              </div>
              <div className="mt-3">
                <p className="font-headline text-xl font-bold uppercase tracking-tight text-white">
                  {activePlatform.label}
                </p>
              </div>
            </div>

            <div className="mt-6 text-center">
              <a
                href={activePlatform.downloadHref}
                download={activePlatform.directDownload}
                target={activePlatform.directDownload ? undefined : "_blank"}
                rel={activePlatform.directDownload ? undefined : "noreferrer"}
                className="inline-flex w-full items-center justify-center gap-3 bg-white px-4 py-4 text-center font-mono text-[12px] font-bold uppercase tracking-[0.24em] text-black transition hover:bg-[#d4d4d4]"
              >
                <LandingIcon name={activePlatform.icon} className="h-4 w-4" />
                {activePlatform.downloadLabel}
              </a>
              <p className="mt-4 font-mono text-xs leading-6 text-terminal-mutedSoft">
                {activePlatform.helperText}
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
