import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const frDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | Espace terminal pour le multi-agent coding",
    homeDescription:
      "Chatminal est un terminal desktop pour exécuter plusieurs sessions IA, les organiser par profils et reprendre un vrai contexte de travail sans repartir de zéro.",
    docsTitle: "Documentation Chatminal",
    docsDescription:
      "Documentation utilisateur pour installer, organiser et utiliser Chatminal.",
  },
  header: {
    home: "Accueil",
    features: "Fonctions",
    downloads: "Téléchargements",
    docs: "Docs",
    downloadCta: "Télécharger",
    languageLabel: "Langue",
  },
  hero: {
    title: "Un espace terminal pour exécuter plusieurs sessions IA en même temps.",
    description:
      "Chatminal garde vos shells, vos runs d'agents et vos setups récurrents bien structurés pour que vous puissiez avancer sur plusieurs branches sans tout écraser dans un seul terminal.",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "Contrôle Multi-Agent",
        description:
          "Exécutez plusieurs sessions IA en parallèle, gardez-les visibles dans un seul arbre et passez d'une branche à l'autre sans perdre la structure du travail.",
      },
      {
        icon: "integration_instructions",
        title: "Sessions Et Profils",
        description:
          "Regroupez les sessions par projet, workflow ou équipe pour que chaque contexte shell reste distinct au lieu de finir dans un seul historique mélangé.",
      },
      {
        icon: "tune",
        title: "Reprendre Le Travail Vite",
        description:
          "Conservez l'historique des sessions, restaurez la forme du workspace et utilisez des commandes de démarrage pour rouvrir les setups récurrents rapidement.",
      },
    ],
  },
  downloads: {
    title: "Télécharger",
    description:
      "Choisissez une plateforme puis copiez la méthode d'installation qui vous convient.",
    copiedLabel: "Copié",
    copyAndRunLabel: "Copier Et Exécuter",
    terminalLabel: "terminal-install",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "Ouvrir Releases",
        helperText: "Choisissez Apple Silicon ou Intel dans GitHub Releases.",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "Installez-le avec Homebrew comme un cask normal.",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "Installe la dernière version stable via le script.",
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
        downloadLabel: "Télécharger Le Tarball",
        helperText: "Téléchargement direct pour Linux x86_64.",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "Installe la dernière version stable via le script.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "Téléchargez directement l'archive Linux depuis la release.",
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
        downloadLabel: "Télécharger Le Zip",
        helperText: "Téléchargement direct pour Windows x64.",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "Ouvrez la page de la dernière release depuis PowerShell.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 Tous droits réservés.",
    home: "Accueil",
    userDocs: "Docs Utilisateur",
    githubRepo: "Repo GitHub",
    statusLog: "Journal Des Releases",
    devDocs: "Docs Dev",
  },
  docs: {
    sidebarTitle: "Sur cette page",
    eyebrow: "Guide utilisateur Chatminal",
    title: "Utilisez Chatminal comme un espace de travail que vous retrouvez, pas comme un terminal jetable.",
    description:
      "Cette page est écrite pour les utilisateurs, pas pour les contributeurs. Elle explique comment installer Chatminal, comment sessions et profils s'articulent, comment les layouts se comportent et ce qu'il se passe quand vous reprenez le travail plus tard.",
    sections: [
      {
        id: "install",
        label: "Installation",
        title: "Installer Chatminal",
        body:
          "Chatminal est un terminal desktop pour celles et ceux qui veulent des sessions shell organisées et faciles à reprendre. Choisissez la méthode qui correspond à votre plateforme et à votre manière de gérer les mises à jour.",
        bullets: [
          "Utilisez le script d'installation pour la voie la plus rapide sur macOS ou Linux.",
          "Utilisez Homebrew sur macOS si vous voulez installer et mettre à jour via brew.",
          "Utilisez GitHub Release si vous préférez télécharger directement l'application ou si vous êtes sous Windows.",
          "Les builds précompilés couvrent actuellement macOS, Linux x86_64 et Windows x64.",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "Installer avec Bash",
            body: "Installe la dernière version stable et reste le chemin le plus direct depuis un terminal.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "Installer avec Homebrew",
            body: "À utiliser sur macOS si vous voulez que Chatminal soit géré comme un cask classique.",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "Télécharger pour Windows",
            body: "Windows est actuellement distribué via le zip de la dernière GitHub Release.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "Premier lancement",
        title: "Ce qu'il se passe au premier lancement",
        body:
          "Quand Chatminal s'ouvre pour la première fois, il démarre votre shell par défaut et vous donne un espace propre pour créer de vraies sessions de travail au lieu d'empiler tout dans un seul onglet terminal.",
        bullets: [
          "Commencez par une session shell fraîche et vérifiez votre environnement par défaut.",
          "Créez des sessions séparées pour des tâches ou dépôts distincts.",
          "Utilisez les profils pour regrouper les sessions liées.",
        ],
      },
      {
        id: "sessions-profiles",
        label: "Sessions",
        title: "Sessions et profils",
        body:
          "Les sessions sont l'unité centrale de Chatminal. Chaque session garde son shell, son répertoire de travail et son état d'activité. Les profils servent à regrouper les sessions par projet, équipe ou workflow.",
        bullets: [
          "Créez une session par tâche, dépôt ou environnement.",
          "Passez d'un profil à l'autre quand vous voulez changer de contexte sans perdre vos sessions en cours.",
          "Renommez les sessions pour que la barre latérale reflète clairement leur rôle.",
          "Utilisez des commandes de démarrage pour les sessions que vous rouvrez souvent.",
        ],
      },
      {
        id: "layouts",
        label: "Layouts",
        title: "Splits et layouts",
        body:
          "Chatminal est conçu pour travailler sur plusieurs sessions à la fois. Les layouts permettent de diviser l'espace, de garder les sessions importantes visibles et de revenir plus tard à une disposition familière.",
        bullets: [
          "Divisez le workspace quand vous avez besoin de logs, de sortie shell et d'une autre tâche en même temps.",
          "Utilisez les layouts pour garder un travail long visible au lieu d'alterner sans cesse.",
          "Les layouts enregistrés facilitent le retour à la même forme de workspace plus tard.",
        ],
      },
      {
        id: "history",
        label: "Historique",
        title: "Historique et reprise",
        body:
          "Chatminal conserve l'état des sessions pour que vous puissiez reprendre le travail sans repartir d'un terminal vide à chaque fois. Cela inclut l'historique des sessions et la structure du workspace.",
        bullets: [
          "L'historique de session peut être conservé pour relire les sorties précédentes.",
          "Rouvrir l'application doit ressembler à une reprise de travail, pas à un redémarrage à zéro.",
          "Si vous voulez repartir proprement, vous pouvez effacer l'historique et réinitialiser le contexte de session.",
        ],
      },
      {
        id: "startup-commands",
        label: "Démarrage",
        title: "Commandes de démarrage",
        body:
          "Si une session commence toujours de la même manière, enregistrez une commande de démarrage. C'est utile pour ouvrir un projet, se rattacher à un outil ou restaurer rapidement un flux shell habituel.",
        bullets: [
          "Utilisez les commandes de démarrage pour les sessions que vous répétez chaque jour.",
          "Gardez-les centrées sur le retour rapide à un état de travail prêt.",
          "Considérez-les comme un confort, pas comme un script de déploiement complet.",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "Questions fréquentes",
        body:
          "Le produit reste aujourd'hui desktop-first et centré sur les sessions. Si vous évaluez si Chatminal correspond à votre manière de travailler, ce sont les questions principales.",
        bullets: [
          "Prend-il en charge plusieurs sessions ? Oui, c'est un élément central du produit.",
          "Puis-je organiser mon travail avec des profils ? Oui, les profils font partie du modèle de workspace persistant.",
          "Se souvient-il des layouts et de l'historique ? Oui, la persistance est intégrée au runtime et au store.",
          "Cette page s'adresse-t-elle aux contributeurs ? Non. Cette page est écrite pour les utilisateurs finaux.",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "Bon retour sur Chatminal",
    tipsTitle: "Conseils pour commencer",
    tipsBody: "Lancez /init pour créer un fichier CLAUDE.md avec des instructions pour ce workspace.",
    recentTitle: "Activité récente",
    recentEmpty: "Aucune activité récente",
    geminiWaiting: "Gemini CLI attend l'authentification dans le workspace chatminal",
  },
};
