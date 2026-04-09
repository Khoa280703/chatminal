export type UserDocsSection = {
  id: string;
  label: string;
  title: string;
  body: string;
  bullets: string[];
  code?: string;
};

export const docsSections: UserDocsSection[] = [
  {
    id: "install",
    label: "Install",
    title: "Install Chatminal",
    body:
      "Chatminal is a desktop terminal for people who want their shell sessions to stay organized and easy to resume. The fastest path is the install script or a prebuilt release.",
    bullets: [
      "Use the install script if you want the quickest setup.",
      "Use GitHub Releases if you prefer downloading a build manually.",
      "Current release artifacts target macOS and Linux.",
    ],
    code: "curl -fsSL https://chatminal.com/install | bash",
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
