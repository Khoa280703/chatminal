"use client";

import Image from "next/image";
import { useCallback, useEffect, useRef, useState } from "react";

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

const claudeTypingStepMs = 41;
const claudeOutputTypingStepMs = 8;
const claudeTypingPauseMs = 1200;
const claudeThinkingStepMs = 420;
const claudeThinkingPauseMs = 900;
const previewBaseWidth = 1024;
const previewBaseHeight = 578;

type AgentMessageStep = {
  kind: "line";
  label: string;
  text: string;
  tone: "user" | "agent" | "status" | "tool" | "muted";
  instant?: boolean;
  ephemeral?: boolean;
};

type AgentThinkingStep = {
  kind: "thinking";
  label: string;
  initialSeconds: number;
  initialTokens: number;
  ticks: number;
};

type AgentConversationStep = AgentMessageStep | AgentThinkingStep;

const claudeConversationScript: AgentConversationStep[] = [
  { kind: "line", label: "you", tone: "user", text: "Hello Claude" },
  {
    kind: "thinking",
    label: "Scanning repo",
    initialSeconds: 3,
    initialTokens: 180,
    ticks: 2,
  },
  {
    kind: "line",
    label: "claude",
    tone: "agent",
    text: "Hi, I'm Claude. What should I work on?",
  },
  {
    kind: "line",
    label: "you",
    tone: "user",
    text: "Create an email service for me.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "I can set up a provider interface, template rendering, and a retry-safe send pipeline.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Entered plan mode",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Read(auth flow, queue layer, env config)",
  },
  {
    kind: "line",
    label: "",
    tone: "muted",
    text: "⎿ Done (9 tool uses · 24.7k tokens · 42s)",
    instant: true,
    ephemeral: true,
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Plan(provider client + templates + delivery jobs)",
  },
  {
    kind: "line",
    label: "",
    tone: "muted",
    text: "⎿ Done (14 tool uses · 35.8k tokens · 1m 03s)",
    instant: true,
    ephemeral: true,
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "I would keep email composition separate from delivery so transactional emails, retries, and provider swaps stay predictable.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Updated plan",
  },
  {
    kind: "line",
    label: "",
    tone: "muted",
    text: "⎿ /plan to preview",
    instant: true,
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Bash(rg -n \"email|mailer|queue|template\" src server packages)",
  },
  {
    kind: "thinking",
    label: "Spelunking",
    initialSeconds: 146,
    initialTokens: 6200,
    ticks: 6,
  },
  {
    kind: "line",
    label: "claude",
    tone: "agent",
    text: "Next I would add the provider adapter, build `sendWelcomeEmail()`, and cover failures with queue-backed retries.",
  },
];

const geminiConversationScript: AgentConversationStep[] = [
  { kind: "line", label: "you", tone: "user", text: "Hello Gemini" },
  {
    kind: "thinking",
    label: "Tracing runtime",
    initialSeconds: 2,
    initialTokens: 140,
    ticks: 2,
  },
  {
    kind: "line",
    label: "gemini",
    tone: "agent",
    text: "Hi, I'm Gemini. What do you want me to inspect?",
  },
  {
    kind: "line",
    label: "you",
    tone: "user",
    text: "Find the mobile overflow bug for me.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "I can inspect the render tree, reproduce the overflow, and narrow it to the component that owns the spacing.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Read(settings modal, sheet layout, mobile breakpoints)",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Inspect(scroll container, min-width rules, action footer)",
  },
  {
    kind: "line",
    label: "",
    tone: "muted",
    text: "⎿ Found overflow caused by nested flex children keeping `min-width: auto`",
    instant: true,
    ephemeral: true,
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Draft fix and verify mobile layout",
  },
  {
    kind: "thinking",
    label: "Synthesizing",
    initialSeconds: 22,
    initialTokens: 540,
    ticks: 5,
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "I would relax the nested flex widths, clamp the modal padding, and keep the footer actions on a tighter wrap rule.",
  },
  {
    kind: "line",
    label: "",
    tone: "agent",
    text: "● Edit(modal content width, footer wrapping, mobile spacing)",
  },
  {
    kind: "line",
    label: "",
    tone: "muted",
    text: "⎿ Tightened spacing and removed the horizontal overflow on narrow viewports",
    instant: true,
    ephemeral: true,
  },
  {
    kind: "line",
    label: "gemini",
    tone: "agent",
    text: "If you want, I can also separate the permanent layout rules from the component-specific fixes so the next modal does not inherit the same bug.",
  },
];

const geminiShortAsciiLogo = `   █████████  ██████████ ██████   ██████ █████ ██████   █████ █████
  ███░░░░░███░░███░░░░░█░░██████ ██████ ░░███ ░░██████ ░░███ ░░███
 ███     ░░░  ░███  █ ░  ░███░█████░███  ░███  ░███░███ ░███  ░███
░███          ░██████    ░███░░███ ░███  ░███  ░███░░███░███  ░███
░███    █████ ░███░░█    ░███ ░░░  ░███  ░███  ░███ ░░██████  ░███
░░███  ░░███  ░███ ░   █ ░███      ░███  ░███  ░███  ░░█████  ░███
 ░░█████████  ██████████ █████     █████ █████ █████  ░░█████ █████
  ░░░░░░░░░  ░░░░░░░░░░ ░░░░░     ░░░░░ ░░░░░ ░░░░░    ░░░░░ ░░░░░`;

const geminiLogoColors = [
  "#a7f3ff",
  "#7dd3fc",
  "#38bdf8",
  "#60a5fa",
  "#60a5fa",
  "#3b82f6",
  "#2563eb",
  "#a7f3ff",
];

const geminiLogoLines = geminiShortAsciiLogo.split("\n");

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
  blueSoft: "\x1b[38;2;167;243;255m",
  blueBright: "\x1b[38;2;96;165;250m",
  blueDeep: "\x1b[38;2;59;130;246m",
  blueGlow: "\x1b[38;2;56;189;248m",
  yellow: "\x1b[38;2;253;224;71m",
};

function buildGeminiTranscript() {
  const lines = [
    `${ansi.blueSoft}   █████████  ██████████ ██████   ██████ █████ ██████   █████ █████${ansi.reset}`,
    `${ansi.blue}  ███░░░░░███░░███░░░░░█░░██████ ██████ ░░███ ░░██████ ░░███ ░░███${ansi.reset}`,
    `${ansi.blueGlow} ███     ░░░  ░███  █ ░  ░███░█████░███  ░███  ░███░███ ░███  ░███${ansi.reset}`,
    `${ansi.blueBright}░███          ░██████    ░███░░███ ░███  ░███  ░███░░███░███  ░███${ansi.reset}`,
    `${ansi.blueBright}░███    █████ ░███░░█    ░███ ░░░  ░███  ░███  ░███ ░░██████  ░███${ansi.reset}`,
    `${ansi.blueDeep}░░███  ░░███  ░███ ░   █ ░███      ░███  ░███  ░███  ░░█████  ░███${ansi.reset}`,
    `${ansi.blue} ░░█████████  ██████████ █████     █████ █████ █████  ░░█████ █████${ansi.reset}`,
    `${ansi.blueSoft}  ░░░░░░░░░  ░░░░░░░░░░ ░░░░░     ░░░░░ ░░░░░ ░░░░░    ░░░░░ ░░░░░${ansi.reset}`,
    "",
    `${ansi.muted}Gemini CLI waiting for auth in chatminal workspace${ansi.reset}`,
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

function buildProtocolSyncTranscript() {
  const prompt = `${ansi.green}khoa2807@chatminal${ansi.reset} ${ansi.blue}apps/landing${ansi.reset} ${ansi.yellow}%${ansi.reset}`;
  const lines = [
    `${prompt} ${ansi.white}git status --short${ansi.reset}`,
    `${ansi.muted} M${ansi.reset} apps/landing/src/components/terminal-window-preview.tsx`,
    `${ansi.muted} M${ansi.reset} apps/desktop/src/termwindow/render/chatminal_sidebar.rs`,
    "",
    `${prompt} ${ansi.white}npm run dev${ansi.reset}`,
    `${ansi.blue}>${ansi.reset} chatminal-landing@0.1.0 dev`,
    `${ansi.blue}>${ansi.reset} next dev`,
    `${ansi.green}✓${ansi.reset} Ready in 1180ms`,
    `${ansi.muted}○${ansi.reset} Local:    http://localhost:3000`,
    "",
    `${prompt} ${ansi.white}rg -n "Protocol_Sync|neural-core" apps/landing/src${ansi.reset}`,
    `${ansi.gray}apps/landing/src/components/terminal-window-preview.tsx${ansi.reset}`,
    `${ansi.gray}apps/desktop/src/termwindow/render/chatminal_sidebar.rs${ansi.reset}`,
    "",
    `${prompt} ${ansi.white}npm run build${ansi.reset}`,
    `${ansi.blue}>${ansi.reset} next build`,
    `${ansi.green}✓${ansi.reset} Compiled successfully`,
    `${ansi.green}✓${ansi.reset} Generating static pages (4/4)`,
    "",
    `${prompt} ${ansi.white}tail -f .next/dev/logs/landing.log${ansi.reset}`,
    `${ansi.muted}[watch]${ansi.reset} route / refreshed after sidebar connector update`,
    `${ansi.muted}[watch]${ansi.reset} route / refreshed after terminal split resize`,
    `${ansi.muted}[watch]${ansi.reset} waiting for next file change...`,
  ];

  return lines.join("\r\n");
}

type TerminalPlaybackStep =
  | { kind: "print"; text: string; delayAfter?: number; chunkSize?: number; charDelay?: number }
  | { kind: "command"; prompt: string; input: string; delayAfter?: number; charDelay?: number }
  | { kind: "pause"; delay: number };

function buildProtocolSyncPlayback(): TerminalPlaybackStep[] {
  const prompt = `${ansi.green}khoa2807@chatminal${ansi.reset} ${ansi.blue}apps/landing${ansi.reset} ${ansi.yellow}%${ansi.reset} `;

  return [
    {
      kind: "command",
      prompt,
      input: "git status --short",
      delayAfter: 180,
      charDelay: 26,
    },
    {
      kind: "print",
      text: `${ansi.muted} M${ansi.reset} apps/landing/src/components/terminal-window-preview.tsx\r\n${ansi.muted} M${ansi.reset} apps/desktop/src/termwindow/render/chatminal_sidebar.rs\r\n\r\n`,
      delayAfter: 340,
      chunkSize: 4,
      charDelay: 10,
    },
    {
      kind: "command",
      prompt,
      input: "npm run dev",
      delayAfter: 150,
      charDelay: 24,
    },
    {
      kind: "print",
      text: `${ansi.blue}>${ansi.reset} chatminal-landing@0.1.0 dev\r\n${ansi.blue}>${ansi.reset} next dev\r\n${ansi.green}✓${ansi.reset} Ready in 1180ms\r\n${ansi.muted}○${ansi.reset} Local:    http://localhost:3000\r\n\r\n`,
      delayAfter: 420,
      chunkSize: 4,
      charDelay: 9,
    },
    {
      kind: "command",
      prompt,
      input: "rg -n \"Protocol_Sync|neural-core\" apps/landing/src",
      delayAfter: 140,
      charDelay: 22,
    },
    {
      kind: "print",
      text: `${ansi.gray}apps/landing/src/components/terminal-window-preview.tsx${ansi.reset}\r\n${ansi.gray}apps/desktop/src/termwindow/render/chatminal_sidebar.rs${ansi.reset}\r\n\r\n`,
      delayAfter: 260,
      chunkSize: 4,
      charDelay: 9,
    },
    {
      kind: "command",
      prompt,
      input: "npm run build",
      delayAfter: 180,
      charDelay: 28,
    },
    {
      kind: "print",
      text: `${ansi.blue}>${ansi.reset} next build\r\n${ansi.green}✓${ansi.reset} Compiled successfully\r\n${ansi.green}✓${ansi.reset} Generating static pages (4/4)\r\n\r\n`,
      delayAfter: 300,
      chunkSize: 4,
      charDelay: 10,
    },
    {
      kind: "command",
      prompt,
      input: "tail -f .next/dev/logs/landing.log",
      delayAfter: 120,
      charDelay: 24,
    },
    {
      kind: "print",
      text: `${ansi.muted}[watch]${ansi.reset} route / refreshed after sidebar connector update\r\n${ansi.muted}[watch]${ansi.reset} route / refreshed after terminal split resize\r\n${ansi.muted}[watch]${ansi.reset} waiting for next file change...\r\n`,
      chunkSize: 4,
      charDelay: 11,
    },
  ];
}

function transcriptForSession(sessionName: string) {
  if (sessionName === "Agent_Debugger") {
    return buildGeminiTranscript();
  }
  if (sessionName === "Protocol_Sync") {
    return buildProtocolSyncTranscript();
  }
  return buildGenericTranscript(sessionName);
}

function shouldAnimateTerminalSession(sessionName: string) {
  return sessionName === "Protocol_Sync";
}

function tokenizeTerminalOutput(output: string) {
  const tokens: string[] = [];
  let index = 0;

  while (index < output.length) {
    if (output[index] === "\x1b" && output[index + 1] === "[") {
      let end = index + 2;
      while (end < output.length && !/[A-Za-z]/.test(output[end])) {
        end += 1;
      }
      if (end < output.length) {
        end += 1;
      }
      tokens.push(output.slice(index, end));
      index = end;
      continue;
    }

    tokens.push(output[index]);
    index += 1;
  }

  return tokens;
}

function isClaudeLogoSession(sessionName: string) {
  return sessionName === "Agent_Architect";
}

function isCustomAgentSession(sessionName: string) {
  return isClaudeLogoSession(sessionName) || isGeminiAgentSession(sessionName);
}

function isGeminiAgentSession(sessionName: string) {
  return sessionName === "Agent_Debugger";
}

function profileForSession(sessionName: string) {
  return profiles.find((profile) =>
    profile.sessions.some((session) => session.name === sessionName),
  );
}

function profileHasJoinedTerminals(profileName: string) {
  return profileName === "neural-core";
}

function isNeuralCoreSession(sessionName: string) {
  return sessionName === "neural-core" || profileForSession(sessionName)?.name === "neural-core";
}

function shouldRenderCustomPreview(sessionName: string) {
  return isCustomAgentSession(sessionName) || isNeuralCoreSession(sessionName);
}

function usePrefersReducedMotion() {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updatePreference = () => {
      setPrefersReducedMotion(mediaQuery.matches);
    };

    updatePreference();
    mediaQuery.addEventListener("change", updatePreference);

    return () => {
      mediaQuery.removeEventListener("change", updatePreference);
    };
  }, []);

  return prefersReducedMotion;
}

function useAgentConversation(script: AgentConversationStep[], disabled = false) {
  const [stepIndex, setStepIndex] = useState(0);
  const [visibleLength, setVisibleLength] = useState(0);
  const [thinkingTick, setThinkingTick] = useState(0);

  useEffect(() => {
    if (disabled) {
      return;
    }

    const currentStep = script[stepIndex];
    if (!currentStep) {
      return;
    }

    const timeout = window.setTimeout(() => {
      if (currentStep.kind === "thinking") {
        if (thinkingTick < currentStep.ticks) {
          setThinkingTick((value) => value + 1);
          return;
        }

        if (stepIndex < script.length - 1) {
          setStepIndex((value) => value + 1);
          setVisibleLength(0);
          setThinkingTick(0);
          return;
        }

        setStepIndex(0);
        setVisibleLength(0);
        setThinkingTick(0);
        return;
      }

      const typingCompleted = visibleLength === currentStep.text.length;
      if (!typingCompleted) {
        if (currentStep.instant) {
          setVisibleLength(currentStep.text.length);
          return;
        }
        setVisibleLength((value) => value + 1);
        return;
      }

      if (stepIndex < script.length - 1) {
        setStepIndex((value) => value + 1);
        setVisibleLength(0);
        setThinkingTick(0);
        return;
      }

      setStepIndex(0);
      setVisibleLength(0);
      setThinkingTick(0);
    }, currentStep.kind === "thinking"
      ? thinkingTick < currentStep.ticks
        ? claudeThinkingStepMs
        : claudeThinkingPauseMs
      : visibleLength === currentStep.text.length
        ? claudeTypingPauseMs
        : currentStep.tone === "user"
          ? claudeTypingStepMs
          : claudeOutputTypingStepMs);

    return () => window.clearTimeout(timeout);
  }, [disabled, script, stepIndex, visibleLength, thinkingTick]);

  if (disabled) {
    return script.map((step) =>
      step.kind === "thinking"
        ? {
            kind: "thinking" as const,
            label: step.label,
            seconds: step.initialSeconds + step.ticks,
            tokens: step.initialTokens + step.ticks * 180,
            isTyping: false,
          }
        : {
            kind: "line" as const,
            label: step.label,
            tone: step.tone,
            instant: step.instant,
            text: step.text,
            isTyping: false,
          },
    );
  }

  return script
    .slice(0, stepIndex + 1)
    .map((step, index, visibleSteps) => {
      const isActiveStep = index === visibleSteps.length - 1;

      if (step.kind === "thinking") {
        if (!isActiveStep) {
          return null;
        }

        return {
          kind: "thinking" as const,
          label: step.label,
          seconds: step.initialSeconds + (isActiveStep ? thinkingTick : step.ticks),
          tokens: step.initialTokens + (isActiveStep ? thinkingTick : step.ticks) * 180,
          isTyping: isActiveStep,
        };
      }

      if (step.ephemeral && !isActiveStep) {
        return null;
      }

      return {
        kind: "line" as const,
        label: step.label,
        tone: step.tone,
        instant: step.instant,
        ephemeral: step.ephemeral,
        text: isActiveStep ? step.text.slice(0, visibleLength) : step.text,
        isTyping: isActiveStep,
      };
    })
    .filter((step) => step !== null);
}

function formatThinkingTokens(tokens: number) {
  if (tokens >= 1000) {
    return `${(tokens / 1000).toFixed(1)}k`;
  }
  return `${tokens}`;
}

function useAutoFollowScroll(trigger: unknown) {
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      const container = scrollRef.current;
      if (!container) {
        return;
      }

      container.scrollTop = container.scrollHeight;
    });

    return () => window.cancelAnimationFrame(frame);
  }, [trigger]);

  return scrollRef;
}

function claudeToneClasses(tone: AgentMessageStep["tone"]) {
  switch (tone) {
    case "agent":
      return {
        label: "text-[#D77757]",
        text: "text-[#f2f2f2]",
        caret: "bg-[#D77757]",
      };
    case "tool":
      return {
        label: "text-[#c98f79]",
        text: "text-[#d8b1a3]",
        caret: "bg-[#D77757]",
      };
    case "status":
      return {
        label: "text-[#aa8f87]",
        text: "text-[#d5c5bf]",
        caret: "bg-[#D77757]",
      };
    case "muted":
      return {
        label: "text-[#8d8d8d]",
        text: "text-[#b5b5b5]",
        caret: "bg-[#8d8d8d]",
      };
    case "user":
    default:
      return {
        label: "text-[#8d8d8d]",
        text: "text-[#f2f2f2]",
        caret: "bg-[#D77757]",
      };
  }
}

function geminiToneClasses(tone: AgentMessageStep["tone"]) {
  switch (tone) {
    case "agent":
      return {
        label: "text-[#60a5fa]",
        text: "text-[#e0f2fe]",
        caret: "bg-[#60a5fa]",
      };
    case "tool":
      return {
        label: "text-[#7dd3fc]",
        text: "text-[#bfdbfe]",
        caret: "bg-[#60a5fa]",
      };
    case "status":
      return {
        label: "text-[#93c5fd]",
        text: "text-[#dbeafe]",
        caret: "bg-[#60a5fa]",
      };
    case "muted":
      return {
        label: "text-[#94a3b8]",
        text: "text-[#cbd5e1]",
        caret: "bg-[#94a3b8]",
      };
    case "user":
    default:
      return {
        label: "text-[#94a3b8]",
        text: "text-[#e0f2fe]",
        caret: "bg-[#60a5fa]",
      };
  }
}

function renderAgentLineText(text: string, isTyping: boolean) {
  if (!text.startsWith("●")) {
    return <span>{text}</span>;
  }

  return (
    <>
      <span aria-hidden="true" className={isTyping ? "agent-status-dot" : undefined}>
        ●
      </span>
      <span>{text.slice(1)}</span>
    </>
  );
}

function ClaudeTerminalPanel({ compact = false }: { compact?: boolean }) {
  usePrefersReducedMotion();
  const conversation = useAgentConversation(claudeConversationScript, false);
  const scrollRef = useAutoFollowScroll(conversation);

  return (
    <div
      className={`flex h-full min-w-0 flex-col overflow-x-hidden bg-black font-mono text-[#d8d8d8] ${
        compact ? "px-2 py-2 md:px-2.5 md:py-2.5" : "px-2 py-2 md:px-2.5 md:py-2.5"
      }`}
    >
      <div
        ref={scrollRef}
        className={`min-h-0 flex-1 overflow-x-hidden overflow-y-auto text-left ${
          compact ? "text-[8.5px] md:text-[9px]" : "text-[9px] md:text-[10px]"
        }`}
      >
        <div className={compact ? "space-y-1" : "space-y-1.5"}>
          <fieldset
            className={`w-full min-w-0 border border-[#D77757] bg-black px-0 pb-0 pt-0 ${
              compact ? "rounded-[4px]" : "rounded-[5px]"
            }`}
          >
            <legend
              className={`ml-1 block px-1 text-left leading-none ${
                compact ? "text-[7px] md:text-[8px]" : "text-[8px] md:text-[9px]"
              }`}
            >
              <span className="text-[#D77757]">Claude Code</span>
              <span className="text-[#b5b5b5]"> v2.1.89</span>
            </legend>

            <div
              className={`relative grid min-w-0 grid-cols-1 ${
                compact ? "pt-1" : "pt-1.5"
              } ${compact ? "" : "xl:grid-cols-[0.95fr_1.05fr]"}`}
            >
              {!compact && (
                <div className="pointer-events-none absolute bottom-2 left-[47.5%] top-2 hidden w-px -translate-x-1/2 bg-[rgba(215,119,87,0.86)] xl:block" />
              )}
              <div
                className={`flex min-w-0 flex-col items-center justify-start text-center ${
                  compact
                    ? "px-2 pb-2 pt-0.5"
                    : "px-2 pb-2.5 pt-0.5 md:px-2.5 md:pb-3 xl:px-5 xl:pb-6 xl:pt-2.5"
                }`}
              >
                <p
                  className={`font-semibold leading-tight text-[#f2f2f2] ${
                    compact ? "text-[9px] md:text-[9.5px]" : "text-[10px] md:text-[11px] xl:text-[15px]"
                  }`}
                  style={compact ? { fontSize: "clamp(8.5px, 0.72vw, 10.5px)" } : undefined}
                >
                  Welcome back Chatminal
                </p>
                <div className={compact ? "my-1" : "my-1.5 md:my-2 xl:my-4"}>
                  <Image
                    src="/claude-logo.svg"
                    alt="Claude logo"
                    width={107}
                    height={63}
                    className={
                      compact
                        ? "h-auto w-[36px] md:w-[40px]"
                        : "h-auto w-[46px] md:w-[54px] xl:w-[107px]"
                    }
                    style={compact ? { width: "clamp(36px, 3vw, 48px)" } : undefined}
                  />
                </div>
                <div
                  className={`w-full space-y-0.5 leading-tight text-[#b5b5b5] ${
                    compact
                      ? "max-w-[112px] text-[7px] md:max-w-[124px] md:text-[7.5px]"
                      : "max-w-[132px] text-[8px] md:max-w-[146px] md:text-[9px] xl:max-w-[180px] xl:text-[13px]"
                  }`}
                  style={compact ? { maxWidth: "clamp(108px, 30vw, 132px)", fontSize: "clamp(7px, 0.58vw, 8.25px)" } : undefined}
                >
                  <p className="break-words">opus-4.6 · API Usage Billing</p>
                  <p className="break-all">~/server/chatminal</p>
                </div>
              </div>

              <div
                className={`flex min-w-0 flex-col text-left ${
                  compact
                    ? "px-2 pb-2 pt-0.5 text-[7.5px] md:px-2.5 md:text-[8px]"
                    : "px-2 pb-2 pt-0.5 text-[8px] md:px-2.5 md:pb-2.5 md:pt-1 md:text-[9px] xl:px-4 xl:pb-4 xl:pt-2.5 xl:text-[12px]"
                }`}
                style={compact ? { fontSize: "clamp(7.4px, 0.62vw, 8.6px)" } : undefined}
              >
                <div className={compact ? "pb-1" : "pb-1"}>
                  <p className={`font-semibold leading-tight text-[rgba(215,119,87,0.86)] ${compact ? "mb-0.5" : "mb-0.5 md:mb-1"}`}>
                    Tips for getting started
                  </p>
                  <p className={`break-words text-[#b5b5b5] ${compact ? "leading-[1.25]" : "leading-[1.35]"}`}>
                    Run /init to create a CLAUDE.md file with instructions for
                    this workspace.
                  </p>
                </div>

                <div className={`border-t border-[#D77757] ${compact ? "pt-1" : "pt-1"}`}>
                  <p className={`font-semibold leading-tight text-[rgba(215,119,87,0.86)] ${compact ? "mb-0.5" : "mb-0.5 md:mb-1"}`}>
                    Recent activity
                  </p>
                  <p className="break-words text-[#b5b5b5]">No recent activity</p>
                </div>
              </div>
            </div>
          </fieldset>

          <div className={compact ? "space-y-1 px-2 py-1" : "space-y-1.5 px-2 py-1 md:px-2.5 md:py-1.5"}>
            {conversation.map((message, index) =>
              message.kind === "thinking" ? (
                <div key={`thinking-${index}`} className={`flex min-w-0 items-start ${compact ? "gap-1.5" : "gap-2"}`}>
                  <span className={`text-[#9a6b5e] ${compact ? "min-w-[42px]" : "min-w-[52px] md:min-w-[58px]"}`}>thinking</span>
                  <p className="min-w-0 break-words text-[#caa08f]">
                    <span>
                      ✶ {message.label}… ({message.seconds}s · ↓{" "}
                      {formatThinkingTokens(message.tokens)} tokens)
                    </span>
                    {message.isTyping && (
                      <span
                        aria-hidden="true"
                        className="claude-command-caret ml-1 inline-block h-[1.05em] w-[7px] align-[-0.18em] bg-[#D77757]"
                      />
                    )}
                  </p>
                </div>
              ) : (
                <div key={`${message.label}-${index}`} className={`flex min-w-0 items-start ${compact ? "gap-1.5" : "gap-2"}`}>
                  <span
                    className={`${compact ? "min-w-[42px]" : "min-w-[52px] md:min-w-[58px]"} ${claudeToneClasses(message.tone).label}`}
                  >
                    {message.label}
                  </span>
                  <p className={`min-w-0 break-words whitespace-pre-wrap ${claudeToneClasses(message.tone).text}`}>
                    {renderAgentLineText(message.text, message.isTyping)}
                    {message.isTyping && (
                      <span
                        aria-hidden="true"
                        className={`claude-command-caret ml-1 inline-block h-[1.05em] w-[7px] align-[-0.18em] ${claudeToneClasses(message.tone).caret}`}
                      />
                    )}
                  </p>
                </div>
              ),
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function GeminiChatPanel({ conversation }: { conversation: ReturnType<typeof useAgentConversation> }) {
  usePrefersReducedMotion();

  return (
    <div className="min-w-0 px-2 py-0.5 font-mono text-[9px] md:px-2.5 md:text-[10px]">
      <div className="space-y-2 text-left">
        {conversation.map((message, index) =>
          message.kind === "thinking" ? (
            <div key={`gemini-thinking-${index}`} className="flex min-w-0 items-start gap-2">
              <span className="min-w-[52px] text-[#60a5fa] md:min-w-[58px]">thinking</span>
              <p className="min-w-0 break-words text-[#93c5fd]">
                <span>
                  ✦ {message.label}… ({message.seconds}s · ↓{" "}
                  {formatThinkingTokens(message.tokens)} tokens)
                </span>
                {message.isTyping && (
                  <span
                    aria-hidden="true"
                    className="claude-command-caret ml-1 inline-block h-[1.05em] w-[7px] align-[-0.18em] bg-[#60a5fa]"
                  />
                )}
              </p>
            </div>
          ) : (
            <div key={`gemini-${message.label}-${index}`} className="flex min-w-0 items-start gap-2">
              <span
                className={`min-w-[52px] md:min-w-[58px] ${geminiToneClasses(message.tone).label}`}
              >
                {message.label}
              </span>
              <p className={`min-w-0 break-words whitespace-pre-wrap ${geminiToneClasses(message.tone).text}`}>
                {renderAgentLineText(message.text, message.isTyping)}
                {message.isTyping && (
                  <span
                    aria-hidden="true"
                    className={`claude-command-caret ml-1 inline-block h-[1.05em] w-[7px] align-[-0.18em] ${geminiToneClasses(message.tone).caret}`}
                  />
                )}
              </p>
            </div>
          ),
        )}
      </div>
    </div>
  );
}

function GeminiTerminalPanel() {
  const conversation = useAgentConversation(geminiConversationScript, false);
  const scrollRef = useAutoFollowScroll(conversation);

  return (
    <div className="flex h-full min-w-0 flex-col overflow-x-hidden bg-black px-2 py-2 font-mono md:px-2.5 md:py-2.5">
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto"
      >
        <div className="min-w-0 space-y-2">
          <pre
            className="m-0 inline-block max-w-none whitespace-pre text-left font-['JetBrains_Mono',monospace] text-[5px] leading-[1] tracking-normal text-[#a7f3ff] md:text-[6px] xl:text-[8px]"
            style={{ fontVariantLigatures: "none" }}
          >
            {geminiLogoLines.map((line, index) => (
              <span key={`${index}-${line}`} className="block" style={{ color: geminiLogoColors[index] }}>
                {line}
              </span>
            ))}
          </pre>
          <div className="break-words text-left text-[9px] text-[#94a3b8] md:text-[10px] xl:text-[12px]">
            Gemini CLI waiting for auth in chatminal workspace
          </div>
          <GeminiChatPanel conversation={conversation} />
        </div>
      </div>
    </div>
  );
}

function NeuralCoreTerminalStack() {
  return (
    <div className="flex h-full bg-black">
      <div className="min-h-0 min-w-0 flex-1 overflow-hidden border-r border-white/10">
        <ClaudeTerminalPanel compact />
      </div>
      <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
        <GeminiTerminalPanel />
      </div>
    </div>
  );
}


function fontSizeForSession(sessionName: string) {
  if (sessionName === "Agent_Debugger" || sessionName === "Protocol_Sync") {
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
  const previewViewportRef = useRef<HTMLDivElement | null>(null);
  const previewFrameRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<XtermInstance | null>(null);
  const fitAddonRef = useRef<FitAddonInstance | null>(null);
  const activeSessionRef = useRef(activeSession);
  const terminalAnimationTimeoutRef = useRef<number | null>(null);
  const terminalAnimationRunIdRef = useRef(0);
  const [previewScale, setPreviewScale] = useState(1);
  const [previewHeight, setPreviewHeight] = useState(previewBaseHeight);

  activeSessionRef.current = activeSession;

  const toggleExpand = (name: string) => {
    setExpanded((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  const stopTerminalAnimation = useCallback(() => {
    terminalAnimationRunIdRef.current += 1;
    if (terminalAnimationTimeoutRef.current !== null) {
      window.clearTimeout(terminalAnimationTimeoutRef.current);
      terminalAnimationTimeoutRef.current = null;
    }
  }, []);

  const scheduleTerminalStep = useCallback((callback: () => void, delay: number) => {
    terminalAnimationTimeoutRef.current = window.setTimeout(callback, delay);
  }, []);

  const playTokenChunk = useCallback((
    term: XtermInstance,
    tokens: string[],
    tokenIndexRef: { value: number },
    chunkSize: number,
  ) => {
    let buffer = "";
    let visibleCount = 0;

    while (tokenIndexRef.value < tokens.length && visibleCount < chunkSize) {
      const token = tokens[tokenIndexRef.value];
      tokenIndexRef.value += 1;
      buffer += token;

      if (!token.startsWith("\x1b[")) {
        visibleCount += 1;
      }
    }

    if (buffer) {
      term.write(buffer);
    }
  }, []);

  const playProtocolSyncSession = useCallback((term: XtermInstance) => {
    stopTerminalAnimation();

    const steps = buildProtocolSyncPlayback();
    const runId = terminalAnimationRunIdRef.current;
    let stepIndex = 0;

    const runStep = () => {
      if (runId !== terminalAnimationRunIdRef.current) {
        return;
      }

      const step = steps[stepIndex];
      if (!step) {
        terminalAnimationTimeoutRef.current = null;
        return;
      }

      if (step.kind === "pause") {
        stepIndex += 1;
        scheduleTerminalStep(runStep, step.delay);
        return;
      }

      if (step.kind === "command") {
        term.write(step.prompt);
        const chars = Array.from(step.input);
        let charIndex = 0;

        const typeCommand = () => {
          if (runId !== terminalAnimationRunIdRef.current) {
            return;
          }

          if (charIndex < chars.length) {
            term.write(chars[charIndex]);
            charIndex += 1;
            scheduleTerminalStep(typeCommand, step.charDelay ?? 24);
            return;
          }

          term.write("\r\n");
          stepIndex += 1;
          scheduleTerminalStep(runStep, step.delayAfter ?? 180);
        };

        typeCommand();
        return;
      }

      const tokens = tokenizeTerminalOutput(step.text);
      const tokenIndexRef = { value: 0 };
      const chunkSize = step.chunkSize ?? 3;
      const charDelay = step.charDelay ?? 10;

      const printChunk = () => {
        if (runId !== terminalAnimationRunIdRef.current) {
          return;
        }

        playTokenChunk(term, tokens, tokenIndexRef, chunkSize);
        if (tokenIndexRef.value < tokens.length) {
          scheduleTerminalStep(printChunk, charDelay);
          return;
        }

        stepIndex += 1;
        scheduleTerminalStep(runStep, step.delayAfter ?? 140);
      };

      printChunk();
    };

    runStep();
  }, [playTokenChunk, scheduleTerminalStep, stopTerminalAnimation]);

  const writeTerminalSession = useCallback((term: XtermInstance, sessionName: string) => {
    const transcript = transcriptForSession(sessionName);

    stopTerminalAnimation();
    if (!shouldAnimateTerminalSession(sessionName)) {
      term.write(transcript);
      return;
    }

    if (sessionName === "Protocol_Sync") {
      playProtocolSyncSession(term);
      return;
    }

    term.write(transcript);
  }, [playProtocolSyncSession, stopTerminalAnimation]);

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

      if (shouldRenderCustomPreview(currentSession)) {
        stopTerminalAnimation();
        return;
      }

      fitAddon.fit();
      writeTerminalSession(term, currentSession);
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
      stopTerminalAnimation();
      resizeObserver?.disconnect();
      terminalRef.current?.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [stopTerminalAnimation, writeTerminalSession]);

  useEffect(() => {
    const term = terminalRef.current;
    const fitAddon = fitAddonRef.current;
    if (!term || !fitAddon) {
      return;
    }

    terminalAnimationRunIdRef.current += 1;
    if (terminalAnimationTimeoutRef.current !== null) {
      window.clearTimeout(terminalAnimationTimeoutRef.current);
      terminalAnimationTimeoutRef.current = null;
    }

    term.options.fontSize = fontSizeForSession(activeSession);
    term.clear();
    term.write("\x1b[H\x1b[2J");

    if (shouldRenderCustomPreview(activeSession)) {
      return;
    }

    fitAddon.fit();
    writeTerminalSession(term, activeSession);
  }, [activeSession, writeTerminalSession]);

  useEffect(() => {
    const viewport = previewViewportRef.current;
    const frame = previewFrameRef.current;
    if (!viewport || !frame) {
      return;
    }

    const updateScale = () => {
      const nextScale = Math.min(1, viewport.clientWidth / frame.offsetWidth);
      setPreviewScale(nextScale);
      setPreviewHeight(frame.offsetHeight * nextScale);
    };

    updateScale();

    const resizeObserver = new ResizeObserver(() => {
      updateScale();
    });

    resizeObserver.observe(viewport);
    resizeObserver.observe(frame);

    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div ref={previewViewportRef} className="w-full max-w-5xl">
      <div
        className="mx-auto"
        style={{
          width: previewBaseWidth * previewScale,
          height: previewHeight,
        }}
      >
        <div
          ref={previewFrameRef}
          className="overflow-hidden border border-white/10 bg-black p-1 shadow-terminal"
          style={{
            width: previewBaseWidth,
            transform: `scale(${previewScale})`,
            transformOrigin: "top left",
          }}
        >
          <div className="flex items-center justify-between border-b border-white/10 bg-[#1a1a1a] px-4 py-2">
            <div className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-terminal-muted">
              <LandingIcon name="terminal" className="h-[18px] w-[18px] text-white" />
              CHATMINAL
            </div>
            <div className="w-6" />
          </div>

          <div className="flex h-[540px] min-w-0 flex-row">
            <aside className="w-1/4 border-r border-white/10 p-4 font-mono text-xs text-[#c6c6c6]">
              <div className="mb-4 flex items-center gap-2 font-bold tracking-widest text-white">
                <LandingIcon name="account_tree" className="h-[18px] w-[18px] text-white" />
                PROJECTS
              </div>
              <div className="space-y-2 text-left">
                {profiles.map((profile) => {
                  const isJoinedTree = profileHasJoinedTerminals(profile.name);
                  const isExpanded = isJoinedTree || expanded[profile.name];

                  return (
                  <div key={profile.name}>
                    <button
                      className={`mb-2 flex w-full items-center gap-2 text-left transition-colors ${
                        activeSession === profile.name
                          ? "font-bold text-white"
                          : "text-white hover:text-white/80"
                      }`}
                      onClick={() => {
                        if (isJoinedTree) {
                          setActiveSession(profile.name);
                        }
                        if (!isJoinedTree) {
                          toggleExpand(profile.name);
                        }
                      }}
                    >
                      <LandingIcon
                        name={isExpanded ? "expand_less" : "expand_more"}
                        className="h-[18px] w-[18px] text-white/80"
                      />
                      <LandingIcon
                        name={profile.icon}
                        className="h-[18px] w-[18px] text-white/80"
                      />
                      {profile.name}
                    </button>

                    {isExpanded && (
                      <div className="relative ml-2 space-y-1 pl-6">
                        <span
                          aria-hidden="true"
                          className="absolute bottom-[10px] left-[7px] top-[10px] w-px bg-white/45"
                        />
                        {profile.sessions.map((session) => {
                          const isJoinedSessionActive =
                            isJoinedTree && isNeuralCoreSession(activeSession);
                          const connectorTone =
                            activeSession === session.name || isJoinedSessionActive
                              ? "bg-white/70"
                              : "bg-white/45";

                          return (
                            <button
                              key={session.name}
                              className={`relative flex w-full items-center gap-2 pl-5 transition-colors ${
                                activeSession === session.name || isJoinedSessionActive
                                  ? "font-bold text-white"
                                  : "text-[#a0a0a0] hover:text-white"
                              }`}
                              onClick={() =>
                                setActiveSession(isJoinedTree ? profile.name : session.name)
                              }
                            >
                              <span
                                aria-hidden="true"
                                className={`absolute left-[7px] top-1/2 h-px w-[12px] -translate-y-1/2 ${connectorTone}`}
                              />
                              <LandingIcon name={session.icon} className="h-[18px] w-[18px]" />
                              {session.name}
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                )})}
              </div>
            </aside>

            <div className="flex min-w-0 flex-1 bg-black p-5">
              <div
                className="relative h-full w-full min-w-0 overflow-hidden rounded-[2px] border border-white/5 bg-black"
              >
                <div
                  ref={containerRef}
                  className={`terminal-xterm-host h-full w-full ${
                    shouldRenderCustomPreview(activeSession) ? "opacity-0" : "opacity-100"
                  }`}
                />
                {isNeuralCoreSession(activeSession) && (
                  <div className="absolute inset-0">
                    <NeuralCoreTerminalStack />
                  </div>
                )}
                {isClaudeLogoSession(activeSession) && (
                  <div className="absolute inset-0">
                    <ClaudeTerminalPanel />
                  </div>
                )}
                {isGeminiAgentSession(activeSession) && (
                  <div className="absolute inset-0">
                    <GeminiTerminalPanel />
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
