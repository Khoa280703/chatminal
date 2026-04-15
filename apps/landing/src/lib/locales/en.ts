import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const enDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | Terminal Workspace For Multi-Agent Coding",
    homeDescription:
      "Chatminal is a desktop terminal workspace for running multiple AI agent sessions, organizing them into profiles, and resuming real work without losing context.",
    docsTitle: "Chatminal Docs",
    docsDescription:
      "End-user documentation for installing, organizing, and using Chatminal.",
  },
  header: {
    home: "Home",
    features: "Features",
    downloads: "Downloads",
    docs: "Docs",
    downloadCta: "Download",
    languageLabel: "Language",
  },
  hero: {
    title: "A terminal workspace for vibe coding.",
    description:
      "Chatminal keeps shell sessions, agent runs, and repeatable setups organized so you can move across parallel work without flattening everything into one terminal.",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "Multi-Agent Control",
        description:
          "Run multiple AI agent sessions side by side, keep them visible in one tree, and switch between concurrent branches without losing structure.",
      },
      {
        icon: "integration_instructions",
        title: "Sessions And Profiles",
        description:
          "Group sessions by project, workflow, or team so each shell context stays separated instead of turning into one long shared terminal history.",
      },
      {
        icon: "tune",
        title: "Resume Real Work Fast",
        description:
          "Keep session history, restore workspace shape, and use startup commands to reopen recurring setups without rebuilding them by hand.",
      },
    ],
  },
  downloads: {
    title: "Download",
    description:
      "Pick a platform, then copy the install path that matches how you work.",
    copiedLabel: "Copied",
    copyAndRunLabel: "Copy And Run",
    terminalLabel: "install-terminal",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "Open Releases",
        helperText: "Choose Apple Silicon or Intel on GitHub Releases.",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "Install it with Homebrew as a normal cask.",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "Install the latest stable release with the installer script.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
        ],
      },
      {
        id: "linux",
        label: "Linux",
        icon: "linux",
        artifact: `${latestReleaseTag} .TAR.GZ`,
        downloadHref: linuxDownloadUrl,
        directDownload: true,
        downloadLabel: "Download Tarball",
        helperText: "Direct download for Linux x86_64.",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "Install the latest stable release with the installer script.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "Download the Linux artifact directly from the release.",
            code: `curl -fL ${linuxDownloadUrl} -o Chatminal-${latestReleaseTag}-linux-x86_64.tar.gz`,
          },
        ],
      },
      {
        id: "windows",
        label: "Windows",
        icon: "windows",
        artifact: `${latestReleaseTag} .ZIP`,
        downloadHref: windowsDownloadUrl,
        directDownload: true,
        downloadLabel: "Download Zip",
        helperText: "Direct download for Windows x64.",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "Open the latest release page from PowerShell.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 All rights reserved.",
    home: "Home",
    userDocs: "User Docs",
    githubRepo: "GitHub Repo",
    statusLog: "Status Log",
    devDocs: "Dev Docs",
  },
  docs: {
    sidebarTitle: "On this page",
    eyebrow: "Chatminal user guide",
    title: "Use Chatminal like a workspace you return to, not a terminal you throw away.",
    description:
      "This page is for users, not contributors. It covers how to install Chatminal, how sessions and profiles fit together, how layouts behave, and what to expect when you come back to work later.",
    sections: [
      {
        id: "install",
        label: "Install",
        title: "Install Chatminal",
        body:
          "Chatminal is a desktop terminal for people who want their shell sessions to stay organized and easy to resume. Choose the install path that matches your platform and how you prefer to manage updates.",
        bullets: [
          "Use the install script if you want the fastest terminal-first setup on macOS or Linux.",
          "Use Homebrew on macOS if you want install and upgrade through brew.",
          "Use the GitHub Release download if you prefer a direct app archive or you are on Windows.",
          "Current prebuilt artifacts cover macOS, Linux x86_64, and Windows x64.",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "Install with Bash",
            body: "This installs the latest stable release and is the quickest path from a terminal.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "Install with Homebrew",
            body: "Use this on macOS if you want Chatminal managed as a normal cask.",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "Download for Windows",
            body: "Windows is currently distributed through the latest GitHub Release zip.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "First launch",
        title: "What happens on first launch",
        body:
          "When Chatminal opens for the first time, it starts with your default shell and gives you a clean place to create sessions for real work instead of stacking everything into a single terminal tab.",
        bullets: [
          "Start with a fresh shell session and confirm your default working environment.",
          "Create separate sessions for separate tasks or repositories.",
          "Use profiles to group related sessions together.",
        ],
      },
      {
        id: "sessions-profiles",
        label: "Sessions",
        title: "Sessions and profiles",
        body:
          "Sessions are the core unit in Chatminal. Each session keeps its own shell, working directory, and activity state. Profiles help you group sessions by project, team, or workflow.",
        bullets: [
          "Create one session per task, repo, or environment.",
          "Move between profiles when you want to switch context without losing your current sessions.",
          "Rename sessions so the sidebar reflects what each one is actually for.",
          "Use startup commands for sessions you reopen often.",
        ],
      },
      {
        id: "layouts",
        label: "Layouts",
        title: "Splits and layouts",
        body:
          "Chatminal is designed for working across multiple sessions at once. Layouts let you split your workspace, keep important sessions visible, and return to a familiar arrangement later.",
        bullets: [
          "Split the workspace when you need logs, shell output, and a second task visible at the same time.",
          "Use layouts to keep long-running work in view instead of switching back and forth.",
          "Saved layouts make it easier to reopen the same workspace shape later.",
        ],
      },
      {
        id: "history",
        label: "History",
        title: "History and resume behavior",
        body:
          "Chatminal persists session state so you can come back to work without starting from a blank terminal every time. That includes session history and the structure of your workspace.",
        bullets: [
          "Session history can be kept so past output is still available when you return.",
          "Reopening the app is meant to feel like resuming work, not relaunching from zero.",
          "If you want a clean slate, you can clear history and reset the session context.",
        ],
      },
      {
        id: "startup-commands",
        label: "Startup",
        title: "Startup commands",
        body:
          "If a session always begins the same way, save a startup command. It is useful for opening a project, attaching to a tool, or restoring a routine shell flow quickly.",
        bullets: [
          "Use startup commands for sessions you repeat every day.",
          "Keep them focused on getting you back to a working state quickly.",
          "Treat them as convenience, not as a full deployment script.",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "Common questions",
        body:
          "The current product path is desktop-first and session-focused. If you are deciding whether Chatminal fits your workflow, these are the questions that matter most.",
        bullets: [
          "Does it support multiple sessions? Yes, that is a core part of the product.",
          "Can I organize work into profiles? Yes, profiles are part of the stored workspace model.",
          "Does it remember layouts and history? Yes, persistence is built into the runtime and store.",
          "Is this page for contributors? No. This page is written for end users, not for people hacking on the repo.",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "Welcome back Chatminal",
    tipsTitle: "Tips for getting started",
    tipsBody: "Run /init to create a CLAUDE.md file with instructions for this workspace.",
    recentTitle: "Recent activity",
    recentEmpty: "No recent activity",
    geminiWaiting: "Gemini CLI waiting for auth in chatminal workspace",
  },
};
