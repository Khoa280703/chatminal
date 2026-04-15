import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const zhCnDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | 面向多 AI 会话的终端工作区",
    homeDescription:
      "Chatminal 是一个桌面终端工作区，用来同时运行多个 AI 会话、按 profile 组织它们，并在回来时继续原来的工作上下文。",
    docsTitle: "Chatminal 文档",
    docsDescription: "面向终端用户的 Chatminal 安装、组织与使用文档。",
  },
  header: {
    home: "首页",
    features: "功能",
    downloads: "下载",
    docs: "文档",
    downloadCta: "下载",
    languageLabel: "语言",
  },
  hero: {
    title: "一个适合 vibe coding 的终端工作区。",
    description:
      "Chatminal 让 shell session、agent 运行与重复性 setup 保持有序，这样你可以在并行工作分支之间切换，而不用把一切都塞进同一个终端里。",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "多 Agent 控制",
        description:
          "并行运行多个 AI 会话，在同一棵树里查看它们，并在多个分支之间切换而不丢失工作结构。",
      },
      {
        icon: "integration_instructions",
        title: "Session 与 Profile",
        description:
          "按项目、工作流或团队组织 session，让每个 shell 上下文保持独立，而不是混成一条很长的终端历史。",
      },
      {
        icon: "tune",
        title: "快速回到真实工作",
        description:
          "保留 session 历史，恢复工作区形态，并通过启动命令快速重新打开常用 setup。",
      },
    ],
  },
  downloads: {
    title: "下载",
    description: "选择平台，然后复制最适合你工作方式的安装命令。",
    copiedLabel: "已复制",
    copyAndRunLabel: "复制并运行",
    terminalLabel: "install-terminal",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "打开 Releases",
        helperText: "在 GitHub Releases 中选择 Apple Silicon 或 Intel。",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "通过 Homebrew 按普通 cask 安装。",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "使用安装脚本安装最新稳定版。",
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
        downloadLabel: "下载 Tarball",
        helperText: "Linux x86_64 直接下载。",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "使用安装脚本安装最新稳定版。",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "直接从 release 下载 Linux 包。",
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
        downloadLabel: "下载 Zip",
        helperText: "Windows x64 直接下载。",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "从 PowerShell 打开最新 release 页面。",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 保留所有权利。",
    home: "首页",
    userDocs: "用户文档",
    githubRepo: "GitHub 仓库",
    statusLog: "版本日志",
    devDocs: "开发文档",
  },
  docs: {
    sidebarTitle: "本页内容",
    eyebrow: "Chatminal 用户指南",
    title: "把 Chatminal 当成你会回来继续使用的工作区，而不是一次性终端。",
    description:
      "这页是写给用户的，不是写给贡献者的。它说明如何安装 Chatminal、session 与 profile 如何配合、layout 如何工作，以及你稍后回来继续工作时会发生什么。",
    sections: [
      {
        id: "install",
        label: "安装",
        title: "安装 Chatminal",
        body:
          "Chatminal 是面向希望 shell session 保持有序且易于继续的用户的桌面终端。请选择与你的平台和更新方式相匹配的安装路径。",
        bullets: [
          "如果你想在 macOS 或 Linux 上获得最快的终端式安装体验，请使用安装脚本。",
          "如果你想通过 brew 安装和升级，请在 macOS 上使用 Homebrew。",
          "如果你更喜欢直接下载应用归档，或者你在 Windows 上，请使用 GitHub Release。",
          "当前预构建产物覆盖 macOS、Linux x86_64 和 Windows x64。",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "使用 Bash 安装",
            body: "这会安装最新稳定版，也是从终端开始最快的方式。",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "使用 Homebrew 安装",
            body: "如果你希望 Chatminal 在 macOS 上像普通 cask 一样被管理，请使用这个方式。",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "下载 Windows 版本",
            body: "Windows 当前通过最新 GitHub Release 的 zip 包分发。",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "首次启动",
        title: "首次启动会发生什么",
        body:
          "Chatminal 第一次打开时，会启动你的默认 shell，并给你一个干净的地方来创建真正用于工作的 session，而不是把所有内容都堆进同一个终端标签页里。",
        bullets: [
          "从一个新的 shell session 开始，并确认你的默认工作环境。",
          "为不同任务或仓库创建独立 session。",
          "使用 profile 来组织相关 session。",
        ],
      },
      {
        id: "sessions-profiles",
        label: "Session",
        title: "Session 与 profile",
        body:
          "Session 是 Chatminal 的核心单元。每个 session 都拥有自己的 shell、工作目录和活动状态。Profile 帮助你按项目、团队或工作流分组这些 session。",
        bullets: [
          "为每个任务、仓库或环境创建一个 session。",
          "当你想切换上下文而不丢失当前 session 时，在 profile 之间切换。",
          "重命名 session，让侧边栏清楚反映它们的用途。",
          "为经常重开的 session 使用启动命令。",
        ],
      },
      {
        id: "layouts",
        label: "布局",
        title: "分屏与布局",
        body:
          "Chatminal 专为同时处理多个 session 而设计。布局让你可以拆分工作区、保持重要 session 可见，并在以后回到熟悉的排列方式。",
        bullets: [
          "当你需要同时看到日志、shell 输出和另一项任务时，拆分工作区。",
          "使用布局让长时间运行的工作一直可见，而不是来回切换。",
          "保存后的布局可以帮助你以后重新打开同样的工作区形状。",
        ],
      },
      {
        id: "history",
        label: "历史",
        title: "历史记录与恢复行为",
        body:
          "Chatminal 会持久化 session 状态，让你回来时不必每次都从空白终端开始。这包括 session 历史以及工作区结构。",
        bullets: [
          "你可以保留 session 历史，这样回来时仍能查看之前的输出。",
          "重新打开应用应该更像继续工作，而不是从零重启。",
          "如果你想要一个干净的起点，可以清除历史并重置 session 上下文。",
        ],
      },
      {
        id: "startup-commands",
        label: "启动",
        title: "启动命令",
        body:
          "如果某个 session 总是以同样的方式开始，就保存一个启动命令。它适合用来打开项目、连接工具，或者快速恢复一个固定 shell 流程。",
        bullets: [
          "为每天都会重复的 session 使用启动命令。",
          "让它们专注于尽快把你带回可工作的状态。",
          "把它当作快捷恢复工具，而不是完整部署脚本。",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "常见问题",
        body:
          "当前产品路线是 desktop-first 和 session-first。如果你在判断 Chatminal 是否适合你的工作流，这些是最关键的问题。",
        bullets: [
          "支持多个 session 吗？支持，这是产品的核心部分。",
          "我可以用 profile 来组织工作吗？可以，profile 是持久化工作区模型的一部分。",
          "它会记住布局和历史吗？会，持久化已经内建在 runtime 和 store 中。",
          "这个页面是给贡献者看的吗？不是。这个页面写给终端用户。",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "欢迎回到 Chatminal",
    tipsTitle: "快速开始提示",
    tipsBody: "运行 /init 来创建包含此工作区说明的 CLAUDE.md 文件。",
    recentTitle: "最近活动",
    recentEmpty: "最近没有活动",
    geminiWaiting: "Gemini CLI 正在 chatminal 工作区中等待授权",
  },
};
