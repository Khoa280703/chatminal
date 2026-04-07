export const githubRepoUrl = "https://github.com/Khoa280703/chatminal";
export const githubDocsUrl =
  "https://github.com/Khoa280703/chatminal/tree/main/docs";
export const githubReleasesUrl =
  "https://github.com/Khoa280703/chatminal/releases";
export const downloadLinks = {
  windows: "https://github.com/Khoa280703/chatminal/releases/download/v0.1.0-test6/Chatminal-v0.1.0-test6-windows-x86_64.zip",
  macos: "https://github.com/Khoa280703/chatminal/releases/download/v0.1.0-test6/Chatminal-v0.1.0-test6-macos.dmg",
  linux: "https://github.com/Khoa280703/chatminal/releases/download/v0.1.0-test6/Chatminal-v0.1.0-test6-linux-x86_64.tar.gz",
};

export const navigationItems = [
  { label: "HOME", href: "#top", active: true },
  { label: "FEATURES", href: "#features" },
  { label: "DOWNLOADS", href: "#downloads" },
  { label: "DOCS", href: githubDocsUrl },
];

export const featureItems = [
  {
    icon: "group_work",
    title: "Multi-Agent Orchestration",
    description:
      "Deploy distinct agents for architecture, debugging, and testing. Manage them simultaneously in a single tree view for maximum cognitive flow.",
  },
  {
    icon: "integration_instructions",
    title: "Deep IDE Integration",
    description:
      "Direct hooks into VS Code, NeoVim, and JetBrains. Your terminal knows your code as well as you do, without manual context dumping.",
  },
  {
    icon: "tune",
    title: "Atomic Sessions",
    description:
      "Every task is an isolated, highly customizable session. Freeze, fork, or merge sessions as you navigate complex architectural transitions.",
  },
];

export const commandSteps = [
  '$ chatminal fork architect --task="refactor"',
  "$ vibe pipe debugger --source=Main_Thread",
  '$ commit --vibe="optimized" --all',
];

export const downloadOptions = [
  { icon: "windows", label: "WINDOWS (X64)", artifact: "v0.1.0-test6 .ZIP", href: downloadLinks.windows },
  { icon: "apple", label: "MACOS", artifact: "v0.1.0-test6 .DMG", href: downloadLinks.macos },
  { icon: "linux", label: "LINUX (X64)", artifact: "v0.1.0-test6 .TAR.GZ", href: downloadLinks.linux },
];

export const footerLinks = [
  { label: "TERMINAL_X", href: "#top" },
  { label: "GITHUB_REPO", href: githubRepoUrl },
  { label: "STATUS_LOG", href: githubReleasesUrl },
  {
    label: "LEGAL_DOCS",
    href: "https://github.com/chatminal/chatminal/blob/main/LICENSE",
  },
];
