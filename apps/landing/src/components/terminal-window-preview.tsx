"use client";

import Image from "next/image";
import { useEffect, useRef, useState } from "react";

import { LandingIcon } from "@/components/landing-icon";

type Session = {
  name: string;
  icon: string;
};

type Profile = {
  name: string;
  icon: string;
  sessions: Session[];
};

type XtermInstance = {
  clear: () => void;
  resize: (columns: number, rows: number) => void;
  write: (data: string) => void;
  open: (element: HTMLElement) => void;
  dispose: () => void;
  options: {
    fontFamily?: string;
    fontSize?: number;
    lineHeight?: number;
    letterSpacing?: number;
  };
};

type FitAddonInstance = {
  fit: () => void;
};

const profiles: Profile[] = [
  {
    name: "vibe-engine",
    icon: "folder",
    sessions: [
      { name: "Agent_Architect", icon: "robot_2" },
      { name: "Agent_Debugger", icon: "robot_2" },
      { name: "Protocol_Sync", icon: "settings_ethernet" },
    ],
  },
  {
    name: "neural-core",
    icon: "folder",
    sessions: [
      { name: "Model_Trainer", icon: "robot_2" },
      { name: "Data_Pipeline", icon: "settings_ethernet" },
    ],
  },
  {
    name: "agent-swarm",
    icon: "folder",
    sessions: [
      { name: "Orchestrator", icon: "robot_2" },
      { name: "Task_Distributor", icon: "settings_ethernet" },
      { name: "Result_Aggregator", icon: "robot_2" },
    ],
  },
];

const ansi = {
  reset: "\x1b[0m",
  white: "\x1b[38;2;255;255;255m",
  gray: "\x1b[38;2;216;216;216m",
  muted: "\x1b[38;2;153;153;153m",
  green: "\x1b[38;2;134;239;172m",
  blue: "\x1b[38;2;125;211;252m",
  yellow: "\x1b[38;2;253;224;71m",
};

function buildGeminiTranscript() {
  const lines = [
    `${ansi.gray}   █████████  ██████████ ██████   ██████ █████ ██████   █████ █████${ansi.reset}`,
    `${ansi.gray}  ███░░░░░███░░███░░░░░█░░██████ ██████ ░░███ ░░██████ ░░███ ░░███${ansi.reset}`,
    `${ansi.gray} ███     ░░░  ░███  █ ░  ░███░█████░███  ░███  ░███░███ ░███  ░███${ansi.reset}`,
    `${ansi.white}░███          ░██████    ░███░░███ ░███  ░███  ░███░░███░███  ░███${ansi.reset}`,
    `${ansi.white}░███    █████ ░███░░█    ░███ ░░░  ░███  ░███  ░███ ░░██████  ░███${ansi.reset}`,
    `${ansi.gray}░░███  ░░███  ░███ ░   █ ░███      ░███  ░███  ░███  ░░█████  ░███${ansi.reset}`,
    `${ansi.gray} ░░█████████  ██████████ █████     █████ █████ █████  ░░█████ █████${ansi.reset}`,
    `${ansi.gray}  ░░░░░░░░░  ░░░░░░░░░░ ░░░░░     ░░░░░ ░░░░░ ░░░░░    ░░░░░ ░░░░░${ansi.reset}`,
    "",
    `${ansi.muted}Gemini CLI waiting for auth in chatminal workspace${ansi.reset}`,
    "",
    `${ansi.white}~${ansi.reset} gemini`,
  ];

  return lines.join("\r\n");
}

function buildGenericTranscript(sessionName: string) {
  const lines = [
    `${ansi.muted}[14:20:01]${ansi.reset} ${ansi.white}${sessionName}${ansi.reset} attached to chatminal workspace`,
    `${ansi.muted}[14:20:02]${ansi.reset} ${ansi.green}✓${ansi.reset} Context loaded from ${ansi.blue}./src${ansi.reset}`,
    `${ansi.muted}[14:20:03]${ansi.reset} ${ansi.green}✓${ansi.reset} Ready for instructions`,
    "",
    `${ansi.white}>_${ansi.reset} help me inspect runtime state and session layout`,
    "",
    `${ansi.yellow}→${ansi.reset} Reading workspace metadata`,
    `${ansi.yellow}→${ansi.reset} Inspecting current terminal topology`,
    `${ansi.yellow}→${ansi.reset} Preparing summary`,
    "",
    `${ansi.white}~${ansi.reset} ${sessionName.toLowerCase()}`,
  ];

  return lines.join("\r\n");
}

function transcriptForSession(sessionName: string) {
  if (sessionName === "Agent_Debugger") {
    return buildGeminiTranscript();
  }
  return buildGenericTranscript(sessionName);
}

function isClaudeLogoSession(sessionName: string) {
  return sessionName === "Agent_Architect";
}

function ClaudeTerminalPanel() {
  return (
    <div className="flex h-full items-start justify-center p-1 md:p-2">
      <fieldset className="w-full max-w-4xl rounded-[8px] border border-[#D77757] bg-black px-0 pb-0 pt-0 font-mono text-[#d8d8d8]">
        <legend className="ml-1 block px-2 text-left text-[11px] leading-none">
          <span className="text-[#D77757]">Claude Code</span>
          <span className="text-[#b5b5b5]"> v2.1.89</span>
        </legend>

        <div className="relative grid grid-cols-1 pt-3 md:grid-cols-[0.95fr_1.05fr]">
          <div className="pointer-events-none absolute bottom-3 left-[47.5%] top-3 hidden w-px -translate-x-1/2 bg-[rgba(215,119,87,0.86)] md:block" />
          <div className="flex flex-col items-center justify-start px-4 pb-5 pt-2 text-center md:px-5 md:pb-6 md:pt-2.5">
            <p className="text-[13px] font-semibold text-[#f2f2f2] md:text-[15px]">
              Welcome back k!
            </p>
            <div className="my-4">
              <Image
                src="/claude-logo.svg"
                alt="Claude logo"
                width={107}
                height={63}
                className="h-auto w-[107px]"
              />
            </div>
            <div className="space-y-1 text-[11px] text-[#b5b5b5] md:text-[13px]">
              <p>qwen3.5-27b · API Usage Billing</p>
              <p>~/development/2026/chatminal</p>
            </div>
          </div>

          <div className="flex flex-col px-3 pb-3 pt-2 text-left text-[11px] md:px-4 md:pb-4 md:pt-2.5 md:text-[12px]">
            <div className="pb-2">
              <p className="mb-1.5 font-semibold text-[rgba(215,119,87,0.86)]">
                Tips for getting started
              </p>
              <p className="leading-relaxed text-[#b5b5b5]">
                Run /init to create a CLAUDE.md file with instructions for this
                workspace.
              </p>
            </div>

            <div className="border-t border-[#D77757] pt-2">
              <p className="mb-1.5 font-semibold text-[rgba(215,119,87,0.86)]">
                Recent activity
              </p>
              <p className="text-[#b5b5b5]">No recent activity</p>
            </div>
          </div>
        </div>
      </fieldset>
    </div>
  );
}

function fontSizeForSession(sessionName: string) {
  if (sessionName === "Agent_Debugger") {
    return 9;
  }
  return 10;
}

export function TerminalWindowPreview() {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({
    "vibe-engine": true,
    "neural-core": true,
    "agent-swarm": true,
  });
  const [activeSession, setActiveSession] = useState<string>("Agent_Architect");

  const containerRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<XtermInstance | null>(null);
  const fitAddonRef = useRef<FitAddonInstance | null>(null);
  const activeSessionRef = useRef(activeSession);

  activeSessionRef.current = activeSession;

  const toggleExpand = (name: string) => {
    setExpanded((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  useEffect(() => {
    let disposed = false;
    let resizeObserver: ResizeObserver | null = null;

    const renderTerminal = () => {
      const term = terminalRef.current;
      const fitAddon = fitAddonRef.current;
      if (!term || !fitAddon) {
        return;
      }

      const currentSession = activeSessionRef.current;
      term.options.fontSize = fontSizeForSession(currentSession);

      term.clear();
      term.write("\x1b[H\x1b[2J");

      if (isClaudeLogoSession(currentSession)) {
        return;
      }

      fitAddon.fit();
      term.write(transcriptForSession(currentSession));
    };

    const setup = async () => {
      const [{ Terminal }, { FitAddon }] = await Promise.all([
        import("@xterm/xterm"),
        import("@xterm/addon-fit"),
      ]);

      if (disposed || !containerRef.current) {
        return;
      }

      const term = new Terminal({
        allowTransparency: false,
        convertEol: true,
        cursorBlink: false,
        disableStdin: true,
        fontFamily: '"JetBrains Mono", monospace',
        fontSize: 10,
        letterSpacing: 0,
        lineHeight: 1.1,
        scrollback: 0,
        theme: {
          background: "#000000",
          cursor: "#ffffff",
          foreground: "#d8d8d8",
          selectionBackground: "rgba(255,255,255,0.14)",
        },
      });

      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.open(containerRef.current);

      terminalRef.current = term as unknown as XtermInstance;
      fitAddonRef.current = fitAddon as unknown as FitAddonInstance;

      renderTerminal();

      resizeObserver = new ResizeObserver(() => {
        renderTerminal();
      });
      resizeObserver.observe(containerRef.current);
    };

    void setup();

    return () => {
      disposed = true;
      resizeObserver?.disconnect();
      terminalRef.current?.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, []);

  useEffect(() => {
    const term = terminalRef.current;
    const fitAddon = fitAddonRef.current;
    if (!term || !fitAddon) {
      return;
    }

    term.options.fontSize = fontSizeForSession(activeSession);
    term.clear();
    term.write("\x1b[H\x1b[2J");

    if (isClaudeLogoSession(activeSession)) {
      return;
    }

    fitAddon.fit();
    term.write(transcriptForSession(activeSession));
  }, [activeSession]);

  return (
    <div className="w-full max-w-5xl overflow-hidden border border-white/10 bg-black p-1 shadow-terminal">
      <div className="flex items-center justify-between border-b border-white/10 bg-[#1a1a1a] px-4 py-2">
        <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-terminal-muted">
          <LandingIcon name="terminal" className="h-[18px] w-[18px] text-white" />
          CHATMINAL
        </div>
        <div className="w-6" />
      </div>

      <div className="flex h-[540px] min-w-0 flex-col md:flex-row">
        <aside className="w-full border-b border-white/10 p-4 font-mono text-xs text-[#c6c6c6] md:w-1/4 md:border-b-0 md:border-r">
          <div className="mb-4 flex items-center gap-2 font-bold tracking-widest text-white">
            <LandingIcon name="account_tree" className="h-[18px] w-[18px] text-white" />
            PROJECTS
          </div>
          <div className="space-y-2 text-left">
            {profiles.map((profile) => (
              <div key={profile.name}>
                <button
                  className="mb-2 flex w-full items-center gap-2 text-left text-white transition-colors hover:text-white/80"
                  onClick={() => toggleExpand(profile.name)}
                >
                  <LandingIcon
                    name={expanded[profile.name] ? "expand_less" : "expand_more"}
                    className="h-[18px] w-[18px] text-white/80"
                  />
                  <LandingIcon
                    name={profile.icon}
                    className="h-[18px] w-[18px] text-white/80"
                  />
                  {profile.name}
                </button>

                {expanded[profile.name] && (
                  <div className="space-y-2 border-l border-white/10 pl-8">
                    {profile.sessions.map((session) => (
                      <button
                        key={session.name}
                        className={`flex w-full items-center gap-2 transition-colors ${
                          activeSession === session.name
                            ? "font-bold text-white"
                            : "text-[#a0a0a0] hover:text-white"
                        }`}
                        onClick={() => setActiveSession(session.name)}
                      >
                        <LandingIcon name={session.icon} className="h-[18px] w-[18px]" />
                        {session.name}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 bg-black p-4 md:p-5">
          <div
            className="relative h-full w-full min-w-0 overflow-hidden rounded-[2px] border border-white/5 bg-black"
          >
            <div
              ref={containerRef}
              className={`terminal-xterm-host h-full w-full ${
                isClaudeLogoSession(activeSession) ? "opacity-0" : "opacity-100"
              }`}
            />
            {isClaudeLogoSession(activeSession) && (
              <div className="absolute inset-0">
                <ClaudeTerminalPanel />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
