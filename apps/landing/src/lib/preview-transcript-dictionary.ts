import type { Locale } from "@/lib/i18n";

export type PreviewTranscriptDictionary = {
  thinkingLabel: string;
  claudeConversation: {
    youLabel: string;
    claudeLabel: string;
    hello: string;
    scanningRepo: string;
    greeting: string;
    taskRequest: string;
    setupPlan: string;
    enteredPlanMode: string;
    readContext: string;
    firstDone: string;
    planStep: string;
    secondDone: string;
    architectureNote: string;
    updatedPlan: string;
    planPreview: string;
    bashSearch: string;
    spelunking: string;
    nextStep: string;
  };
  geminiConversation: {
    youLabel: string;
    geminiLabel: string;
    hello: string;
    tracingRuntime: string;
    greeting: string;
    taskRequest: string;
    inspectPlan: string;
    readModal: string;
    inspectScroll: string;
    foundOverflow: string;
    draftFix: string;
    synthesizing: string;
    fixPlan: string;
    editStep: string;
    fixedResult: string;
    followUp: string;
  };
  genericTranscript: {
    attachedToWorkspace: string;
    contextLoadedFrom: string;
    readyForInstructions: string;
    helpInspectRuntime: string;
    readingWorkspaceMetadata: string;
    inspectingTerminalTopology: string;
    preparingSummary: string;
  };
  installTranscript: {
    macosGuide: string;
    macosHomebrewNote: string;
    macosInstallerNote: string;
    macosReleaseNote: string;
    linuxGuide: string;
    linuxInstallerNote: string;
    linuxTarballNote: string;
    windowsGuide: string;
    windowsReleaseNote: string;
    windowsExtractNote: string;
  };
  installPlayback: {
    macosTitle: string;
    macosDownloading: string;
    macosLinking: string;
    macosLaunched: string;
    macosUpgradeNote: string;
    linuxTitle: string;
    linuxPreparing: string;
    linuxInstalledTo: string;
    linuxTreeReady: string;
    linuxStableNote: string;
    windowsTitle: string;
    windowsOpening: string;
    windowsExtracted: string;
    windowsLaunchNote: string;
  };
};

const dictionaries: Record<Locale, PreviewTranscriptDictionary> = {
  en: {
    thinkingLabel: "thinking",
    claudeConversation: {
      youLabel: "you",
      claudeLabel: "claude",
      hello: "Hello Claude",
      scanningRepo: "Scanning repo",
      greeting: "Hi, I'm Claude. What should I work on?",
      taskRequest: "Create an email service for me.",
      setupPlan:
        "I can set up a provider interface, template rendering, and a retry-safe send pipeline.",
      enteredPlanMode: "● Entered plan mode",
      readContext: "● Read(auth flow, queue layer, env config)",
      firstDone: "⎿ Done (9 tool uses · 24.7k tokens · 42s)",
      planStep: "● Plan(provider client + templates + delivery jobs)",
      secondDone: "⎿ Done (14 tool uses · 35.8k tokens · 1m 03s)",
      architectureNote:
        "I would keep email composition separate from delivery so transactional emails, retries, and provider swaps stay predictable.",
      updatedPlan: "● Updated plan",
      planPreview: "⎿ /plan to preview",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "Spelunking",
      nextStep:
        "Next I would add the provider adapter, build `sendWelcomeEmail()`, and cover failures with queue-backed retries.",
    },
    geminiConversation: {
      youLabel: "you",
      geminiLabel: "gemini",
      hello: "Hello Gemini",
      tracingRuntime: "Tracing runtime",
      greeting: "Hi, I'm Gemini. What do you want me to inspect?",
      taskRequest: "Find the mobile overflow bug for me.",
      inspectPlan:
        "I can inspect the render tree, reproduce the overflow, and narrow it to the component that owns the spacing.",
      readModal: "● Read(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● Inspect(scroll container, min-width rules, action footer)",
      foundOverflow:
        "⎿ Found overflow caused by nested flex children keeping `min-width: auto`",
      draftFix: "● Draft fix and verify mobile layout",
      synthesizing: "Synthesizing",
      fixPlan:
        "I would relax the nested flex widths, clamp the modal padding, and keep the footer actions on a tighter wrap rule.",
      editStep: "● Edit(modal content width, footer wrapping, mobile spacing)",
      fixedResult:
        "⎿ Tightened spacing and removed the horizontal overflow on narrow viewports",
      followUp:
        "If you want, I can also separate the permanent layout rules from the component-specific fixes so the next modal does not inherit the same bug.",
    },
    genericTranscript: {
      attachedToWorkspace: "attached to chatminal workspace",
      contextLoadedFrom: "Context loaded from",
      readyForInstructions: "Ready for instructions",
      helpInspectRuntime: "help me inspect runtime state and session layout",
      readingWorkspaceMetadata: "Reading workspace metadata",
      inspectingTerminalTopology: "Inspecting current terminal topology",
      preparingSummary: "Preparing summary",
    },
    installTranscript: {
      macosGuide: "Chatminal install guide for macOS",
      macosHomebrewNote: "Install with Homebrew if you want a clean update path.",
      macosInstallerNote: "Use the installer script if you want the fastest setup.",
      macosReleaseNote: "Pick Apple Silicon or Intel manually from the latest release.",
      linuxGuide: "Chatminal install guide for Linux",
      linuxInstallerNote: "Installs the latest stable Linux x86_64 release.",
      linuxTarballNote: "Use the tarball if you want a manual install flow.",
      windowsGuide: "Chatminal install guide for Windows",
      windowsReleaseNote: "Download the latest Windows zip from the release page.",
      windowsExtractNote: "Unzip it, then launch Chatminal from the extracted folder.",
    },
    installPlayback: {
      macosTitle: "Install Chatminal on macOS",
      macosDownloading: "Downloading Chatminal.app",
      macosLinking: "Linking chatminal into /opt/homebrew/bin",
      macosLaunched: "Chatminal launched",
      macosUpgradeNote: "Use Homebrew for future upgrades.",
      linuxTitle: "Install Chatminal on Linux",
      linuxPreparing: "Preparing Chatminal v0.1.5 for linux/x86_64",
      linuxInstalledTo: "Installed chatminal to ~/.local/bin/chatminal",
      linuxTreeReady: "Session tree ready",
      linuxStableNote: "Latest stable release installed.",
      windowsTitle: "Install Chatminal on Windows",
      windowsOpening: "Opening latest release page in your browser...",
      windowsExtracted: "Archive extracted to .\\chatminal",
      windowsLaunchNote: "Launch Chatminal from the extracted folder.",
    },
  },
  vi: {
    thinkingLabel: "đang nghĩ",
    claudeConversation: {
      youLabel: "bạn",
      claudeLabel: "claude",
      hello: "Chào Claude",
      scanningRepo: "Đang quét repo",
      greeting: "Chào, tôi là Claude. Tôi nên làm gì?",
      taskRequest: "Tạo cho tôi một email service.",
      setupPlan:
        "Tôi có thể dựng provider interface, phần render template và pipeline gửi có retry an toàn.",
      enteredPlanMode: "● Đã vào plan mode",
      readContext: "● Đã đọc(auth flow, queue layer, env config)",
      firstDone: "⎿ Xong (9 lần dùng tool · 24.7k tokens · 42s)",
      planStep: "● Lập plan(provider client + templates + delivery jobs)",
      secondDone: "⎿ Xong (14 lần dùng tool · 35.8k tokens · 1m 03s)",
      architectureNote:
        "Tôi sẽ tách phần soạn email khỏi phần gửi để email giao dịch, retry và thay provider vẫn ổn định.",
      updatedPlan: "● Đã cập nhật plan",
      planPreview: "⎿ /plan để xem trước",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "Đang đào code",
      nextStep:
        "Tiếp theo tôi sẽ thêm provider adapter, dựng `sendWelcomeEmail()` và bọc lỗi bằng retry qua queue.",
    },
    geminiConversation: {
      youLabel: "bạn",
      geminiLabel: "gemini",
      hello: "Chào Gemini",
      tracingRuntime: "Đang lần runtime",
      greeting: "Chào, tôi là Gemini. Bạn muốn tôi kiểm tra gì?",
      taskRequest: "Tìm giúp tôi lỗi overflow trên mobile.",
      inspectPlan:
        "Tôi có thể soi render tree, tái hiện lỗi overflow và thu hẹp về đúng component đang giữ spacing.",
      readModal: "● Đã đọc(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● Đã kiểm tra(scroll container, min-width rules, action footer)",
      foundOverflow:
        "⎿ Đã thấy overflow do các flex children lồng nhau vẫn giữ `min-width: auto`",
      draftFix: "● Soạn bản sửa và kiểm tra lại layout mobile",
      synthesizing: "Đang tổng hợp",
      fixPlan:
        "Tôi sẽ nới width của nested flex, clamp padding của modal và giữ footer actions wrap chặt hơn.",
      editStep: "● Sửa(modal content width, footer wrapping, mobile spacing)",
      fixedResult: "⎿ Đã siết spacing và bỏ overflow ngang trên viewport hẹp",
      followUp:
        "Nếu muốn, tôi có thể tách rule layout cố định ra khỏi các fix theo component để modal sau không dính lại lỗi này.",
    },
    genericTranscript: {
      attachedToWorkspace: "đã gắn vào workspace chatminal",
      contextLoadedFrom: "Đã nạp context từ",
      readyForInstructions: "Sẵn sàng nhận chỉ dẫn",
      helpInspectRuntime: "giúp tôi kiểm tra runtime state và layout session",
      readingWorkspaceMetadata: "Đang đọc metadata của workspace",
      inspectingTerminalTopology: "Đang kiểm tra topology terminal hiện tại",
      preparingSummary: "Đang chuẩn bị tóm tắt",
    },
    installTranscript: {
      macosGuide: "Hướng dẫn cài Chatminal cho macOS",
      macosHomebrewNote: "Cài bằng Homebrew nếu bạn muốn đường nâng cấp gọn gàng.",
      macosInstallerNote: "Dùng script cài đặt nếu bạn muốn cách dựng nhanh nhất.",
      macosReleaseNote: "Tự chọn Apple Silicon hoặc Intel trong release mới nhất.",
      linuxGuide: "Hướng dẫn cài Chatminal cho Linux",
      linuxInstallerNote: "Cài bản stable Linux x86_64 mới nhất.",
      linuxTarballNote: "Dùng tarball nếu bạn muốn tự cài thủ công.",
      windowsGuide: "Hướng dẫn cài Chatminal cho Windows",
      windowsReleaseNote: "Tải file zip Windows mới nhất từ trang release.",
      windowsExtractNote: "Giải nén rồi mở Chatminal từ thư mục đã extract.",
    },
    installPlayback: {
      macosTitle: "Cài Chatminal trên macOS",
      macosDownloading: "Đang tải Chatminal.app",
      macosLinking: "Đang link chatminal vào /opt/homebrew/bin",
      macosLaunched: "Chatminal đã mở",
      macosUpgradeNote: "Về sau hãy nâng cấp bằng Homebrew.",
      linuxTitle: "Cài Chatminal trên Linux",
      linuxPreparing: "Đang chuẩn bị Chatminal v0.1.5 cho linux/x86_64",
      linuxInstalledTo: "Đã cài chatminal vào ~/.local/bin/chatminal",
      linuxTreeReady: "Cây session đã sẵn sàng",
      linuxStableNote: "Đã cài bản stable mới nhất.",
      windowsTitle: "Cài Chatminal trên Windows",
      windowsOpening: "Đang mở trang release mới nhất trong trình duyệt...",
      windowsExtracted: "Đã giải nén archive vào .\\chatminal",
      windowsLaunchNote: "Hãy mở Chatminal từ thư mục đã giải nén.",
    },
  },
  fr: {
    thinkingLabel: "analyse",
    claudeConversation: {
      youLabel: "vous",
      claudeLabel: "claude",
      hello: "Bonjour Claude",
      scanningRepo: "Analyse du repo",
      greeting: "Bonjour, je suis Claude. Sur quoi dois-je travailler ?",
      taskRequest: "Crée-moi un service email.",
      setupPlan:
        "Je peux mettre en place une interface provider, le rendu des templates et un pipeline d'envoi robuste avec retry.",
      enteredPlanMode: "● Entré en mode plan",
      readContext: "● Lu(auth flow, queue layer, env config)",
      firstDone: "⎿ Terminé (9 appels outil · 24.7k tokens · 42s)",
      planStep: "● Plan(provider client + templates + delivery jobs)",
      secondDone: "⎿ Terminé (14 appels outil · 35.8k tokens · 1m 03s)",
      architectureNote:
        "Je séparerais la composition d'email de la livraison pour garder les emails transactionnels, les retries et les changements de provider prévisibles.",
      updatedPlan: "● Plan mis à jour",
      planPreview: "⎿ /plan pour prévisualiser",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "Exploration",
      nextStep:
        "Ensuite j'ajouterais l'adaptateur provider, je construirais `sendWelcomeEmail()` et je couvrirais les erreurs avec des retries via la queue.",
    },
    geminiConversation: {
      youLabel: "vous",
      geminiLabel: "gemini",
      hello: "Bonjour Gemini",
      tracingRuntime: "Analyse du runtime",
      greeting: "Bonjour, je suis Gemini. Que voulez-vous que j'inspecte ?",
      taskRequest: "Trouve-moi le bug d'overflow mobile.",
      inspectPlan:
        "Je peux inspecter l'arbre de rendu, reproduire l'overflow et remonter jusqu'au composant qui gère l'espacement.",
      readModal: "● Lu(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● Inspecté(scroll container, min-width rules, action footer)",
      foundOverflow:
        "⎿ Overflow trouvé : des enfants flex imbriqués gardent `min-width: auto`",
      draftFix: "● Préparer le correctif et vérifier le layout mobile",
      synthesizing: "Synthèse",
      fixPlan:
        "Je relâcherais les largeurs flex imbriquées, je limiterais le padding du modal et je garderais les actions du footer sur une règle de wrap plus serrée.",
      editStep: "● Édition(modal content width, footer wrapping, mobile spacing)",
      fixedResult:
        "⎿ Espacement resserré et overflow horizontal supprimé sur les viewports étroits",
      followUp:
        "Si vous voulez, je peux aussi séparer les règles de layout permanentes des correctifs spécifiques au composant pour éviter que le prochain modal hérite du même bug.",
    },
    genericTranscript: {
      attachedToWorkspace: "attaché au workspace chatminal",
      contextLoadedFrom: "Contexte chargé depuis",
      readyForInstructions: "Prêt pour les instructions",
      helpInspectRuntime: "aide-moi à inspecter l'état runtime et le layout des sessions",
      readingWorkspaceMetadata: "Lecture des métadonnées du workspace",
      inspectingTerminalTopology: "Inspection de la topologie actuelle du terminal",
      preparingSummary: "Préparation du résumé",
    },
    installTranscript: {
      macosGuide: "Guide d'installation Chatminal pour macOS",
      macosHomebrewNote: "Installez avec Homebrew si vous voulez une voie de mise à jour propre.",
      macosInstallerNote: "Utilisez le script d'installation si vous voulez la mise en place la plus rapide.",
      macosReleaseNote: "Choisissez Apple Silicon ou Intel manuellement sur la dernière release.",
      linuxGuide: "Guide d'installation Chatminal pour Linux",
      linuxInstallerNote: "Installe la dernière release stable Linux x86_64.",
      linuxTarballNote: "Utilisez le tarball si vous préférez une installation manuelle.",
      windowsGuide: "Guide d'installation Chatminal pour Windows",
      windowsReleaseNote: "Téléchargez le dernier zip Windows depuis la page release.",
      windowsExtractNote: "Décompressez-le puis lancez Chatminal depuis le dossier extrait.",
    },
    installPlayback: {
      macosTitle: "Installer Chatminal sur macOS",
      macosDownloading: "Téléchargement de Chatminal.app",
      macosLinking: "Lien de chatminal vers /opt/homebrew/bin",
      macosLaunched: "Chatminal lancé",
      macosUpgradeNote: "Utilisez Homebrew pour les prochaines mises à jour.",
      linuxTitle: "Installer Chatminal sur Linux",
      linuxPreparing: "Préparation de Chatminal v0.1.5 pour linux/x86_64",
      linuxInstalledTo: "chatminal installé dans ~/.local/bin/chatminal",
      linuxTreeReady: "Arbre de sessions prêt",
      linuxStableNote: "Dernière release stable installée.",
      windowsTitle: "Installer Chatminal sur Windows",
      windowsOpening: "Ouverture de la dernière page release dans votre navigateur...",
      windowsExtracted: "Archive extraite dans .\\chatminal",
      windowsLaunchNote: "Lancez Chatminal depuis le dossier extrait.",
    },
  },
  "zh-cn": {
    thinkingLabel: "思考中",
    claudeConversation: {
      youLabel: "你",
      claudeLabel: "claude",
      hello: "你好 Claude",
      scanningRepo: "正在扫描仓库",
      greeting: "你好，我是 Claude。要我做什么？",
      taskRequest: "帮我做一个邮件服务。",
      setupPlan: "我可以先搭 provider 接口、模板渲染，再加一个支持重试的发送流水线。",
      enteredPlanMode: "● 已进入 plan mode",
      readContext: "● 已读取(auth flow, queue layer, env config)",
      firstDone: "⎿ 已完成（9 次工具调用 · 24.7k tokens · 42s）",
      planStep: "● 制定计划(provider client + templates + delivery jobs)",
      secondDone: "⎿ 已完成（14 次工具调用 · 35.8k tokens · 1m 03s）",
      architectureNote:
        "我会把邮件内容组装和投递拆开，这样事务邮件、重试和切换 provider 都更可控。",
      updatedPlan: "● 已更新计划",
      planPreview: "⎿ 用 /plan 预览",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "深入排查",
      nextStep:
        "接下来我会补上 provider adapter，实现 `sendWelcomeEmail()`，再用 queue-backed retry 覆盖失败场景。",
    },
    geminiConversation: {
      youLabel: "你",
      geminiLabel: "gemini",
      hello: "你好 Gemini",
      tracingRuntime: "正在追踪 runtime",
      greeting: "你好，我是 Gemini。你想让我检查什么？",
      taskRequest: "帮我找移动端 overflow bug。",
      inspectPlan: "我可以检查渲染树、复现 overflow，并定位到真正控制 spacing 的组件。",
      readModal: "● 已读取(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● 已检查(scroll container, min-width rules, action footer)",
      foundOverflow: "⎿ 已发现 overflow 由嵌套 flex 子元素仍保持 `min-width: auto` 导致",
      draftFix: "● 起草修复并验证 mobile layout",
      synthesizing: "正在整合",
      fixPlan: "我会放宽嵌套 flex 宽度、限制 modal padding，并让 footer actions 用更紧的换行规则。",
      editStep: "● 编辑(modal content width, footer wrapping, mobile spacing)",
      fixedResult: "⎿ 已收紧 spacing，并移除窄视口下的横向 overflow",
      followUp:
        "如果你愿意，我还可以把长期布局规则和组件级修复拆开，避免下一个 modal 继承同样的问题。",
    },
    genericTranscript: {
      attachedToWorkspace: "已连接到 chatminal 工作区",
      contextLoadedFrom: "已从此处加载上下文",
      readyForInstructions: "已准备好接收指令",
      helpInspectRuntime: "帮我检查 runtime 状态和 session 布局",
      readingWorkspaceMetadata: "正在读取工作区元数据",
      inspectingTerminalTopology: "正在检查当前终端拓扑",
      preparingSummary: "正在准备总结",
    },
    installTranscript: {
      macosGuide: "macOS 版 Chatminal 安装指南",
      macosHomebrewNote: "如果你希望升级路径更干净，请使用 Homebrew 安装。",
      macosInstallerNote: "如果你想要最快的安装方式，请使用安装脚本。",
      macosReleaseNote: "请在最新 release 中手动选择 Apple Silicon 或 Intel。",
      linuxGuide: "Linux 版 Chatminal 安装指南",
      linuxInstallerNote: "安装最新稳定版 Linux x86_64 release。",
      linuxTarballNote: "如果你想手动安装，请使用 tarball。",
      windowsGuide: "Windows 版 Chatminal 安装指南",
      windowsReleaseNote: "从 release 页面下载最新的 Windows zip。",
      windowsExtractNote: "解压后从解压目录启动 Chatminal。",
    },
    installPlayback: {
      macosTitle: "在 macOS 上安装 Chatminal",
      macosDownloading: "正在下载 Chatminal.app",
      macosLinking: "正在把 chatminal 链接到 /opt/homebrew/bin",
      macosLaunched: "Chatminal 已启动",
      macosUpgradeNote: "后续升级请使用 Homebrew。",
      linuxTitle: "在 Linux 上安装 Chatminal",
      linuxPreparing: "正在为 linux/x86_64 准备 Chatminal v0.1.5",
      linuxInstalledTo: "已将 chatminal 安装到 ~/.local/bin/chatminal",
      linuxTreeReady: "session 树已准备好",
      linuxStableNote: "已安装最新稳定版。",
      windowsTitle: "在 Windows 上安装 Chatminal",
      windowsOpening: "正在浏览器中打开最新 release 页面...",
      windowsExtracted: "已将压缩包解压到 .\\chatminal",
      windowsLaunchNote: "请从解压目录启动 Chatminal。",
    },
  },
  ru: {
    thinkingLabel: "думаю",
    claudeConversation: {
      youLabel: "вы",
      claudeLabel: "claude",
      hello: "Привет, Claude",
      scanningRepo: "Сканирую repo",
      greeting: "Привет, я Claude. Над чем мне поработать?",
      taskRequest: "Сделай мне email service.",
      setupPlan:
        "Я могу собрать provider interface, рендеринг шаблонов и безопасный send pipeline с retry.",
      enteredPlanMode: "● Вошёл в plan mode",
      readContext: "● Прочитал(auth flow, queue layer, env config)",
      firstDone: "⎿ Готово (9 вызовов tool · 24.7k tokens · 42s)",
      planStep: "● План(provider client + templates + delivery jobs)",
      secondDone: "⎿ Готово (14 вызовов tool · 35.8k tokens · 1m 03s)",
      architectureNote:
        "Я бы отделил сборку письма от доставки, чтобы transactional emails, retry и смена provider оставались предсказуемыми.",
      updatedPlan: "● План обновлён",
      planPreview: "⎿ /plan для предпросмотра",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "Копаю глубже",
      nextStep:
        "Дальше я бы добавил provider adapter, собрал `sendWelcomeEmail()` и покрыл сбои retry через очередь.",
    },
    geminiConversation: {
      youLabel: "вы",
      geminiLabel: "gemini",
      hello: "Привет, Gemini",
      tracingRuntime: "Разбираю runtime",
      greeting: "Привет, я Gemini. Что именно проверить?",
      taskRequest: "Найди мне mobile overflow bug.",
      inspectPlan:
        "Я могу проверить render tree, воспроизвести overflow и сузить проблему до компонента, который держит spacing.",
      readModal: "● Прочитал(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● Проверил(scroll container, min-width rules, action footer)",
      foundOverflow: "⎿ Нашёл overflow: вложенные flex children сохраняют `min-width: auto`",
      draftFix: "● Подготовить фикс и проверить mobile layout",
      synthesizing: "Собираю вывод",
      fixPlan:
        "Я бы ослабил ширины у nested flex, зажал padding модалки и оставил footer actions на более плотном wrap-правиле.",
      editStep: "● Правка(modal content width, footer wrapping, mobile spacing)",
      fixedResult: "⎿ Поджал spacing и убрал горизонтальный overflow на узких viewport",
      followUp:
        "Если хотите, я ещё могу отделить постоянные layout rules от компонентных фиксов, чтобы следующая modal не унаследовала ту же проблему.",
    },
    genericTranscript: {
      attachedToWorkspace: "подключён к workspace chatminal",
      contextLoadedFrom: "Context загружен из",
      readyForInstructions: "Готов к инструкциям",
      helpInspectRuntime: "помоги мне проверить runtime state и layout сессий",
      readingWorkspaceMetadata: "Читаю metadata workspace",
      inspectingTerminalTopology: "Проверяю текущую topology терминала",
      preparingSummary: "Готовлю summary",
    },
    installTranscript: {
      macosGuide: "Гид по установке Chatminal для macOS",
      macosHomebrewNote: "Ставьте через Homebrew, если хотите чистый путь обновлений.",
      macosInstallerNote: "Используйте install script, если нужен самый быстрый старт.",
      macosReleaseNote: "Выберите Apple Silicon или Intel вручную в последнем release.",
      linuxGuide: "Гид по установке Chatminal для Linux",
      linuxInstallerNote: "Ставит последний stable Linux x86_64 release.",
      linuxTarballNote: "Используйте tarball, если хотите ручной flow установки.",
      windowsGuide: "Гид по установке Chatminal для Windows",
      windowsReleaseNote: "Скачайте последний Windows zip со страницы release.",
      windowsExtractNote: "Распакуйте архив и запустите Chatminal из извлечённой папки.",
    },
    installPlayback: {
      macosTitle: "Установить Chatminal на macOS",
      macosDownloading: "Скачивание Chatminal.app",
      macosLinking: "Линкуем chatminal в /opt/homebrew/bin",
      macosLaunched: "Chatminal запущен",
      macosUpgradeNote: "Для следующих обновлений используйте Homebrew.",
      linuxTitle: "Установить Chatminal на Linux",
      linuxPreparing: "Подготовка Chatminal v0.1.5 для linux/x86_64",
      linuxInstalledTo: "chatminal установлен в ~/.local/bin/chatminal",
      linuxTreeReady: "Дерево сессий готово",
      linuxStableNote: "Последний stable release установлен.",
      windowsTitle: "Установить Chatminal на Windows",
      windowsOpening: "Открываю страницу последнего release в браузере...",
      windowsExtracted: "Архив распакован в .\\chatminal",
      windowsLaunchNote: "Запустите Chatminal из распакованной папки.",
    },
  },
  hi: {
    thinkingLabel: "सोच रहा है",
    claudeConversation: {
      youLabel: "आप",
      claudeLabel: "claude",
      hello: "नमस्ते Claude",
      scanningRepo: "repo स्कैन हो रहा है",
      greeting: "नमस्ते, मैं Claude हूँ। मुझे किस पर काम करना चाहिए?",
      taskRequest: "मेरे लिए एक email service बना दो।",
      setupPlan:
        "मैं provider interface, template rendering और retry-safe send pipeline सेट कर सकता हूँ।",
      enteredPlanMode: "● plan mode में गया",
      readContext: "● पढ़ा(auth flow, queue layer, env config)",
      firstDone: "⎿ पूरा (9 tool uses · 24.7k tokens · 42s)",
      planStep: "● Plan(provider client + templates + delivery jobs)",
      secondDone: "⎿ पूरा (14 tool uses · 35.8k tokens · 1m 03s)",
      architectureNote:
        "मैं email composition को delivery से अलग रखूँगा ताकि transactional emails, retries और provider swap predictable रहें।",
      updatedPlan: "● plan अपडेट हुआ",
      planPreview: "⎿ preview के लिए /plan",
      bashSearch: '● Bash(rg -n "email|mailer|queue|template" src server packages)',
      spelunking: "गहराई से देख रहा है",
      nextStep:
        "अगले चरण में मैं provider adapter जोड़ूँगा, `sendWelcomeEmail()` बनाऊँगा और failures को queue-backed retries से cover करूँगा।",
    },
    geminiConversation: {
      youLabel: "आप",
      geminiLabel: "gemini",
      hello: "नमस्ते Gemini",
      tracingRuntime: "runtime ट्रेस हो रहा है",
      greeting: "नमस्ते, मैं Gemini हूँ। आप क्या inspect कराना चाहते हैं?",
      taskRequest: "मेरे लिए mobile overflow bug ढूँढो।",
      inspectPlan:
        "मैं render tree inspect कर सकता हूँ, overflow reproduce कर सकता हूँ और उसे उस component तक सीमित कर सकता हूँ जो spacing own करता है।",
      readModal: "● पढ़ा(settings modal, sheet layout, mobile breakpoints)",
      inspectScroll: "● inspect किया(scroll container, min-width rules, action footer)",
      foundOverflow:
        "⎿ overflow मिला क्योंकि nested flex children अब भी `min-width: auto` रखे हुए हैं",
      draftFix: "● fix ड्राफ्ट और mobile layout verify",
      synthesizing: "समेट रहा है",
      fixPlan:
        "मैं nested flex widths ढीली करूँगा, modal padding clamp करूँगा और footer actions के wrap rule को tighter रखूँगा।",
      editStep: "● Edit(modal content width, footer wrapping, mobile spacing)",
      fixedResult: "⎿ spacing tighten हुई और narrow viewport पर horizontal overflow हट गया",
      followUp:
        "अगर आप चाहें तो मैं permanent layout rules को component-specific fixes से अलग भी कर सकता हूँ ताकि अगला modal वही bug inherit न करे।",
    },
    genericTranscript: {
      attachedToWorkspace: "chatminal workspace से जुड़ गया",
      contextLoadedFrom: "Context यहाँ से लोड हुआ",
      readyForInstructions: "निर्देशों के लिए तैयार",
      helpInspectRuntime: "runtime state और session layout inspect करने में मेरी मदद करो",
      readingWorkspaceMetadata: "workspace metadata पढ़ रहा है",
      inspectingTerminalTopology: "वर्तमान terminal topology inspect कर रहा है",
      preparingSummary: "summary तैयार कर रहा है",
    },
    installTranscript: {
      macosGuide: "macOS के लिए Chatminal install guide",
      macosHomebrewNote: "अगर clean upgrade path चाहिए तो Homebrew से install करें।",
      macosInstallerNote: "अगर सबसे तेज setup चाहिए तो installer script इस्तेमाल करें।",
      macosReleaseNote: "नवीनतम release से Apple Silicon या Intel मैन्युअली चुनें।",
      linuxGuide: "Linux के लिए Chatminal install guide",
      linuxInstallerNote: "नवीनतम stable Linux x86_64 release install करता है।",
      linuxTarballNote: "अगर manual install flow चाहिए तो tarball इस्तेमाल करें।",
      windowsGuide: "Windows के लिए Chatminal install guide",
      windowsReleaseNote: "release page से नवीनतम Windows zip डाउनलोड करें।",
      windowsExtractNote: "इसे unzip करें, फिर extracted folder से Chatminal चलाएँ।",
    },
    installPlayback: {
      macosTitle: "macOS पर Chatminal install करें",
      macosDownloading: "Chatminal.app डाउनलोड हो रहा है",
      macosLinking: "chatminal को /opt/homebrew/bin में link किया जा रहा है",
      macosLaunched: "Chatminal शुरू हो गया",
      macosUpgradeNote: "आगे के upgrades के लिए Homebrew इस्तेमाल करें।",
      linuxTitle: "Linux पर Chatminal install करें",
      linuxPreparing: "linux/x86_64 के लिए Chatminal v0.1.5 तैयार हो रहा है",
      linuxInstalledTo: "chatminal ~/.local/bin/chatminal में install हो गया",
      linuxTreeReady: "session tree तैयार है",
      linuxStableNote: "नवीनतम stable release install हो गई।",
      windowsTitle: "Windows पर Chatminal install करें",
      windowsOpening: "ब्राउज़र में नवीनतम release page खुल रहा है...",
      windowsExtracted: "archive .\\chatminal में extract हो गया",
      windowsLaunchNote: "extracted folder से Chatminal चलाएँ।",
    },
  },
};

export function getPreviewTranscriptDictionary(locale: Locale): PreviewTranscriptDictionary {
  return dictionaries[locale] ?? dictionaries.en;
}
