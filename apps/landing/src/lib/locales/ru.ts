import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const ruDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | Терминальное рабочее пространство для multi-agent coding",
    homeDescription:
      "Chatminal — это desktop-терминал для одновременного запуска нескольких AI-сессий, организации их по профилям и возврата к работе без потери контекста.",
    docsTitle: "Документация Chatminal",
    docsDescription:
      "Пользовательская документация по установке, организации и использованию Chatminal.",
  },
  header: {
    home: "Главная",
    features: "Возможности",
    downloads: "Загрузка",
    docs: "Документация",
    downloadCta: "Скачать",
    languageLabel: "Язык",
  },
  hero: {
    title: "Терминальное рабочее пространство для vibe coding.",
    description:
      "Chatminal держит shell-сессии, agent run и повторяемые setup в порядке, чтобы вы могли двигаться по параллельным веткам работы, не сваливая всё в один терминал.",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "Управление Несколькими Agent",
        description:
          "Запускайте несколько AI-сессий параллельно, держите их в одном дереве и переключайтесь между ветками, не теряя структуру работы.",
      },
      {
        icon: "integration_instructions",
        title: "Сессии И Профили",
        description:
          "Группируйте сессии по проектам, workflow или командам, чтобы каждый shell-контекст оставался отдельным, а не превращался в одну длинную смешанную историю терминала.",
      },
      {
        icon: "tune",
        title: "Быстро Возвращайтесь К Работе",
        description:
          "Сохраняйте историю сессий, восстанавливайте форму workspace и используйте startup-команды, чтобы быстро открывать привычные setup заново.",
      },
    ],
  },
  downloads: {
    title: "Загрузка",
    description: "Выберите платформу и скопируйте способ установки, который подходит именно вам.",
    copiedLabel: "Скопировано",
    copyAndRunLabel: "Скопировать И Запустить",
    terminalLabel: "install-terminal",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "Открыть Releases",
        helperText: "Выберите Apple Silicon или Intel на странице GitHub Releases.",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "Установите через Homebrew как обычный cask.",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "Установите последнюю стабильную версию через install script.",
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
        downloadLabel: "Скачать Tarball",
        helperText: "Прямая загрузка для Linux x86_64.",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "Установите последнюю стабильную версию через install script.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "Скачайте Linux-архив напрямую из release.",
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
        downloadLabel: "Скачать Zip",
        helperText: "Прямая загрузка для Windows x64.",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "Откройте страницу последнего release из PowerShell.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 Все права защищены.",
    home: "Главная",
    userDocs: "Пользовательская Документация",
    githubRepo: "GitHub Репозиторий",
    statusLog: "Журнал Релизов",
    devDocs: "Документация Для Разработчиков",
  },
  docs: {
    sidebarTitle: "На этой странице",
    eyebrow: "Руководство пользователя Chatminal",
    title: "Используйте Chatminal как рабочее пространство, к которому вы возвращаетесь, а не как одноразовый терминал.",
    description:
      "Эта страница написана для пользователей, а не для контрибьюторов. Здесь объясняется, как установить Chatminal, как связаны sessions и profiles, как работают layouts и чего ожидать, когда вы возвращаетесь к работе позже.",
    sections: [
      {
        id: "install",
        label: "Установка",
        title: "Установите Chatminal",
        body:
          "Chatminal — это desktop-терминал для тех, кто хочет держать shell-сессии организованными и легко возобновляемыми. Выберите способ установки, который соответствует вашей платформе и подходу к обновлениям.",
        bullets: [
          "Используйте install script, если хотите самый быстрый terminal-first путь на macOS или Linux.",
          "Используйте Homebrew на macOS, если хотите устанавливать и обновлять через brew.",
          "Используйте GitHub Release, если предпочитаете прямую загрузку архива приложения или работаете на Windows.",
          "Готовые сборки сейчас доступны для macOS, Linux x86_64 и Windows x64.",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "Установка через Bash",
            body: "Устанавливает последнюю стабильную версию и остаётся самым быстрым путём из терминала.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "Установка через Homebrew",
            body: "Используйте на macOS, если хотите, чтобы Chatminal управлялся как обычный cask.",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "Загрузка для Windows",
            body: "Windows сейчас распространяется через zip-файл последнего GitHub Release.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "Первый запуск",
        title: "Что происходит при первом запуске",
        body:
          "Когда Chatminal открывается впервые, он запускает ваш shell по умолчанию и даёт чистое место для создания рабочих сессий вместо того, чтобы складывать всё в одну терминальную вкладку.",
        bullets: [
          "Начните с новой shell-сессии и проверьте своё рабочее окружение по умолчанию.",
          "Создавайте отдельные сессии для отдельных задач или репозиториев.",
          "Используйте profiles, чтобы группировать связанные сессии.",
        ],
      },
      {
        id: "sessions-profiles",
        label: "Sessions",
        title: "Sessions и profiles",
        body:
          "Sessions — базовая единица в Chatminal. Каждая session хранит собственный shell, рабочий каталог и состояние активности. Profiles помогают группировать sessions по проекту, команде или workflow.",
        bullets: [
          "Создавайте одну session на задачу, репозиторий или окружение.",
          "Переключайтесь между profiles, когда хотите сменить контекст, не теряя текущие sessions.",
          "Переименовывайте sessions, чтобы боковая панель честно отражала их назначение.",
          "Используйте startup-команды для sessions, которые открываете часто.",
        ],
      },
      {
        id: "layouts",
        label: "Layouts",
        title: "Разделения и layouts",
        body:
          "Chatminal спроектирован для работы сразу с несколькими sessions. Layouts позволяют делить workspace, держать важные sessions на виду и позже возвращаться к знакомой раскладке.",
        bullets: [
          "Разделяйте workspace, когда вам нужно одновременно видеть логи, вывод shell и ещё одну задачу.",
          "Используйте layouts, чтобы держать длительные процессы на виду, а не переключаться туда-сюда.",
          "Сохранённые layouts упрощают повторное открытие той же формы workspace позже.",
        ],
      },
      {
        id: "history",
        label: "История",
        title: "История и возобновление",
        body:
          "Chatminal сохраняет состояние sessions, чтобы вы могли вернуться к работе без старта с пустого терминала каждый раз. Это включает историю sessions и структуру workspace.",
        bullets: [
          "История session может сохраняться, чтобы старый вывод оставался доступен при возвращении.",
          "Повторное открытие приложения должно ощущаться как продолжение работы, а не как запуск с нуля.",
          "Если нужен чистый старт, можно очистить историю и сбросить контекст session.",
        ],
      },
      {
        id: "startup-commands",
        label: "Запуск",
        title: "Startup-команды",
        body:
          "Если session всегда начинается одинаково, сохраните startup-команду. Это удобно для открытия проекта, подключения к инструменту или быстрого восстановления привычного shell-потока.",
        bullets: [
          "Используйте startup-команды для sessions, которые повторяются каждый день.",
          "Держите их сфокусированными на быстром возврате в рабочее состояние.",
          "Считайте их удобством, а не полноценным deployment script.",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "Частые вопросы",
        body:
          "Текущий путь продукта — desktop-first и session-focused. Если вы решаете, подходит ли Chatminal под ваш workflow, именно эти вопросы самые важные.",
        bullets: [
          "Поддерживает ли он несколько sessions? Да, это одна из ключевых частей продукта.",
          "Можно ли организовать работу по profiles? Да, profiles — часть сохраняемой модели workspace.",
          "Запоминает ли он layouts и history? Да, persistence встроен в runtime и store.",
          "Эта страница для контрибьюторов? Нет. Она написана для конечных пользователей.",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "С возвращением в Chatminal",
    tipsTitle: "Советы для старта",
    tipsBody: "Запустите /init, чтобы создать файл CLAUDE.md с инструкциями для этого workspace.",
    recentTitle: "Недавняя активность",
    recentEmpty: "Недавней активности нет",
    geminiWaiting: "Gemini CLI ждёт авторизацию в рабочем пространстве chatminal",
  },
};
