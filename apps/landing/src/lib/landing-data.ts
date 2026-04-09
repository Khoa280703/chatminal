export const githubRepoUrl = "https://github.com/Khoa280703/chatminal";
export const githubDeveloperDocsUrl =
  "https://github.com/Khoa280703/chatminal/tree/main/docs";
export const githubReleasesUrl =
  "https://github.com/Khoa280703/chatminal/releases";
export const latestReleaseTag = "v0.1.1";
export const downloadLinks = {
  windows: `https://github.com/Khoa280703/chatminal/releases/download/${latestReleaseTag}/Chatminal-${latestReleaseTag}-windows-x86_64.zip`,
  macos: githubReleasesUrl,
  linux: `https://github.com/Khoa280703/chatminal/releases/download/${latestReleaseTag}/Chatminal-${latestReleaseTag}-linux-x86_64.tar.gz`,
};

export const navigationItems = [
  { label: "HOME", href: "/" },
  { label: "FEATURES", href: "/#features" },
  { label: "DOWNLOADS", href: "/#downloads" },
  { label: "DOCS", href: "/docs" },
];

export const featureItems = [
  {
    icon: "robot_2",
    title: "Multi-Agent Control",
    description:
      "Run multiple AI agent sessions at once, keep them connected in the sidebar tree, and move across parallel branches without losing the shape of the job.",
  },
  {
    icon: "integration_instructions",
    title: "Sessions And Profiles",
    description:
      "Organize shell sessions by task, project, or workflow, group them into profiles, and keep each context separate instead of piling everything into one terminal.",
  },
  {
    icon: "tune",
    title: "Resume And Restart Fast",
    description:
      "Restore workspace state when you come back, keep useful history, and use startup commands to reopen recurring setups with less manual repetition.",
  },
];

export const commandSteps = [
  '$ chatminal fork architect --task="refactor"',
  "$ vibe pipe debugger --source=Main_Thread",
  '$ commit --vibe="optimized" --all',
];

export const downloadOptions = [
  {
    icon: "windows",
    label: "WINDOWS (X64)",
    artifact: `${latestReleaseTag} .ZIP`,
    href: downloadLinks.windows,
    directDownload: true,
  },
  {
    icon: "apple",
    label: "MACOS",
    artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
    href: downloadLinks.macos,
    directDownload: false,
  },
  {
    icon: "linux",
    label: "LINUX (X64)",
    artifact: `${latestReleaseTag} .TAR.GZ`,
    href: downloadLinks.linux,
    directDownload: true,
  },
];

export const footerLinks = [
  { label: "HOME", href: "/" },
  { label: "USER_DOCS", href: "/docs" },
  { label: "GITHUB_REPO", href: githubRepoUrl },
  { label: "STATUS_LOG", href: githubReleasesUrl },
  {
    label: "DEV_DOCS",
    href: githubDeveloperDocsUrl,
  },
];
