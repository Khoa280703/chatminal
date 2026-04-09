export type UserDocsSection = {
  id: string;
  label: string;
  title: string;
  body: string;
  bullets: string[];
  code?: string;
  methods?: {
    id: string;
    label: string;
    title: string;
    body: string;
    code: string;
  }[];
};

export const docsSections: UserDocsSection[] = [
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
];

export const docsQuickLinks = docsSections.map(({ id, label }) => ({ id, label }));

export const docsLinks = {
  releases: "https://github.com/Khoa280703/chatminal/releases",
  repo: "https://github.com/Khoa280703/chatminal",
  issues: "https://github.com/Khoa280703/chatminal/issues",
} as const;
