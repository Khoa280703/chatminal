import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}"],
  theme: {
    extend: {
      fontFamily: {
        headline: ["var(--font-space-grotesk)"],
        body: ["var(--font-space-grotesk)"],
        mono: ["var(--font-jetbrains-mono)"],
      },
      colors: {
        terminal: {
          background: "#000000",
          panel: "#0a0a0a",
          panelSoft: "#111111",
          panelStrong: "#1a1a1a",
          muted: "#b0b0b0",
          mutedSoft: "#8d8d8d",
          text: "#e2e2e2",
          border: "rgba(255,255,255,0.1)",
        },
      },
      boxShadow: {
        terminal: "0 0 40px -10px rgba(255, 255, 255, 0.06)",
      },
      letterSpacing: {
        terminal: "0.3em",
      },
    },
  },
  plugins: [],
};

export default config;
