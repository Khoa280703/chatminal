import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const hiDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | मल्टी-AI सेशन के लिए टर्मिनल वर्कस्पेस",
    homeDescription:
      "Chatminal एक डेस्कटॉप टर्मिनल वर्कस्पेस है जहाँ आप कई AI सेशन साथ चला सकते हैं, उन्हें प्रोफाइल में व्यवस्थित कर सकते हैं और बाद में उसी काम के संदर्भ में वापस आ सकते हैं।",
    docsTitle: "Chatminal डॉक्स",
    docsDescription:
      "Chatminal को इंस्टॉल, व्यवस्थित और उपयोग करने के लिए एंड-यूज़र डॉक्यूमेंटेशन।",
  },
  header: {
    home: "होम",
    features: "फ़ीचर",
    downloads: "डाउनलोड",
    docs: "डॉक्स",
    downloadCta: "डाउनलोड",
    languageLabel: "भाषा",
  },
  hero: {
    title: "vibe coding के लिए एक टर्मिनल वर्कस्पेस।",
    description:
      "Chatminal shell sessions, agent runs और बार-बार इस्तेमाल होने वाले setups को व्यवस्थित रखता है ताकि आप समानांतर काम की शाखाओं में बिना सब कुछ एक ही टर्मिनल में ठूँसे आगे बढ़ सकें।",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "मल्टी-Agent कंट्रोल",
        description:
          "कई AI सेशन साथ चलाइए, उन्हें एक ही ट्री में देखिए और समानांतर शाखाओं के बीच बिना संरचना खोए स्विच कीजिए।",
      },
      {
        icon: "integration_instructions",
        title: "सेशन और प्रोफाइल",
        description:
          "सेशन को प्रोजेक्ट, workflow या टीम के हिसाब से समूहित कीजिए ताकि हर shell context अलग रहे और सब कुछ एक लंबी मिली-जुली terminal history न बन जाए।",
      },
      {
        icon: "tune",
        title: "काम पर जल्दी लौटें",
        description:
          "सेशन history रखें, workspace shape restore करें और startup commands से बार-बार खुलने वाले setups जल्दी वापस पाएँ।",
      },
    ],
  },
  downloads: {
    title: "डाउनलोड",
    description:
      "अपना प्लेटफ़ॉर्म चुनिए, फिर वही install path कॉपी कीजिए जो आपके काम के तरीके से मेल खाता हो।",
    copiedLabel: "कॉपी हो गया",
    copyAndRunLabel: "कॉपी करके चलाएँ",
    terminalLabel: "install-terminal",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "Releases खोलें",
        helperText: "GitHub Releases पर Apple Silicon या Intel चुनें।",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "इसे Homebrew से एक सामान्य cask की तरह इंस्टॉल करें।",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "installer script से latest stable release इंस्टॉल करें।",
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
        downloadLabel: "Tarball डाउनलोड करें",
        helperText: "Linux x86_64 के लिए सीधा डाउनलोड।",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "installer script से latest stable release इंस्टॉल करें।",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "Linux artifact सीधे release से डाउनलोड करें।",
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
        downloadLabel: "Zip डाउनलोड करें",
        helperText: "Windows x64 के लिए सीधा डाउनलोड।",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "PowerShell से latest release page खोलें।",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 सर्वाधिकार सुरक्षित।",
    home: "होम",
    userDocs: "यूज़र डॉक्स",
    githubRepo: "GitHub रिपो",
    statusLog: "रिलीज़ लॉग",
    devDocs: "डेव डॉक्स",
  },
  docs: {
    sidebarTitle: "इस पेज पर",
    eyebrow: "Chatminal उपयोगकर्ता मार्गदर्शिका",
    title: "Chatminal को ऐसे इस्तेमाल करें जैसे यह आपका लौटकर आने वाला workspace हो, कोई फेंक देने वाला terminal नहीं।",
    description:
      "यह पेज contributors के लिए नहीं, users के लिए है। इसमें बताया गया है कि Chatminal कैसे इंस्टॉल करें, sessions और profiles कैसे साथ काम करते हैं, layouts कैसे व्यवहार करते हैं और बाद में काम पर लौटने पर क्या उम्मीद रखनी चाहिए।",
    sections: [
      {
        id: "install",
        label: "इंस्टॉल",
        title: "Chatminal इंस्टॉल करें",
        body:
          "Chatminal उन लोगों के लिए desktop terminal है जो चाहते हैं कि उनके shell sessions व्यवस्थित रहें और आसानी से resume हों। वही install path चुनें जो आपके platform और update workflow से मेल खाए।",
        bullets: [
          "अगर आप macOS या Linux पर सबसे तेज terminal-first setup चाहते हैं तो install script इस्तेमाल करें।",
          "अगर आप macOS पर brew के ज़रिए install और upgrade चाहते हैं तो Homebrew इस्तेमाल करें।",
          "अगर आप direct app archive पसंद करते हैं या Windows पर हैं तो GitHub Release डाउनलोड का उपयोग करें।",
          "अभी prebuilt artifacts macOS, Linux x86_64 और Windows x64 के लिए उपलब्ध हैं।",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "Bash से इंस्टॉल करें",
            body: "यह latest stable release इंस्टॉल करता है और terminal से शुरू करने का सबसे तेज़ रास्ता है।",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "Homebrew से इंस्टॉल करें",
            body: "macOS पर इसे चुनें अगर आप चाहते हैं कि Chatminal एक सामान्य cask की तरह manage हो।",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "Windows के लिए डाउनलोड",
            body: "Windows फिलहाल latest GitHub Release zip के ज़रिए वितरित होता है।",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "पहला लॉन्च",
        title: "पहले लॉन्च पर क्या होता है",
        body:
          "जब Chatminal पहली बार खुलता है, यह आपका default shell शुरू करता है और आपको असली काम के लिए sessions बनाने की साफ जगह देता है, बजाय इसके कि सब कुछ एक ही terminal tab में भर दिया जाए।",
        bullets: [
          "एक fresh shell session से शुरू करें और अपना default working environment जाँचें।",
          "अलग tasks या repositories के लिए अलग sessions बनाएँ।",
          "related sessions को group करने के लिए profiles का उपयोग करें।",
        ],
      },
      {
        id: "sessions-profiles",
        label: "सेशन",
        title: "सेशन और प्रोफाइल",
        body:
          "Chatminal में session मूल इकाई है। हर session अपना shell, working directory और activity state रखता है। Profiles आपको sessions को project, team या workflow के हिसाब से समूहित करने में मदद करते हैं।",
        bullets: [
          "हर task, repo या environment के लिए एक session बनाएँ।",
          "जब आप context बदलना चाहें लेकिन current sessions न खोना चाहें, तब profiles के बीच जाएँ।",
          "sessions का नाम बदलें ताकि sidebar साफ दिखाए कि हर session किस काम के लिए है।",
          "जो sessions आप बार-बार खोलते हैं उनके लिए startup commands इस्तेमाल करें।",
        ],
      },
      {
        id: "layouts",
        label: "लेआउट",
        title: "Splits और layouts",
        body:
          "Chatminal एक साथ कई sessions पर काम करने के लिए बनाया गया है। Layouts आपको workspace split करने, महत्वपूर्ण sessions को दृश्य में रखने और बाद में उसी परिचित arrangement पर लौटने देते हैं।",
        bullets: [
          "जब आपको logs, shell output और दूसरी task एक साथ देखनी हो तो workspace split करें।",
          "लंबे समय तक चलने वाले काम को दृश्य में रखने के लिए layouts इस्तेमाल करें, बार-बार switch करने के बजाय।",
          "saved layouts बाद में वही workspace shape दोबारा खोलना आसान बनाते हैं।",
        ],
      },
      {
        id: "history",
        label: "इतिहास",
        title: "इतिहास और resume व्यवहार",
        body:
          "Chatminal session state को persist करता है ताकि आप हर बार खाली terminal से शुरू किए बिना काम पर लौट सकें। इसमें session history और workspace structure दोनों शामिल हैं।",
        bullets: [
          "session history रखी जा सकती है ताकि वापस आने पर पुराना output उपलब्ध रहे।",
          "app दोबारा खोलना ऐसा महसूस होना चाहिए जैसे काम resume हो रहा हो, न कि सब कुछ zero से शुरू हो रहा हो।",
          "अगर आप clean slate चाहते हैं, तो history साफ़ कर सकते हैं और session context reset कर सकते हैं।",
        ],
      },
      {
        id: "startup-commands",
        label: "स्टार्टअप",
        title: "Startup commands",
        body:
          "अगर कोई session हमेशा एक ही तरीके से शुरू होता है, तो startup command सहेज लें। यह project खोलने, किसी tool से attach होने या routine shell flow जल्दी restore करने के लिए उपयोगी है।",
        bullets: [
          "जो sessions आप रोज़ दोहराते हैं उनके लिए startup commands इस्तेमाल करें।",
          "इन्हें इस बात पर केंद्रित रखें कि आपको जल्दी से working state में वापस लाना है।",
          "इन्हें convenience समझें, full deployment script नहीं।",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "सामान्य प्रश्न",
        body:
          "अभी product direction desktop-first और session-focused है। अगर आप तय कर रहे हैं कि Chatminal आपके workflow के लिए सही है या नहीं, तो ये सबसे महत्वपूर्ण प्रश्न हैं।",
        bullets: [
          "क्या यह कई sessions सपोर्ट करता है? हाँ, यह product का core हिस्सा है।",
          "क्या मैं profiles में काम व्यवस्थित कर सकता हूँ? हाँ, profiles stored workspace model का हिस्सा हैं।",
          "क्या यह layouts और history याद रखता है? हाँ, persistence runtime और store में built-in है।",
          "क्या यह पेज contributors के लिए है? नहीं। यह पेज end users के लिए लिखा गया है।",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "Chatminal में फिर से स्वागत है",
    tipsTitle: "शुरू करने के लिए सुझाव",
    tipsBody: "इस workspace के निर्देशों के साथ CLAUDE.md file बनाने के लिए /init चलाएँ।",
    recentTitle: "हाल की गतिविधि",
    recentEmpty: "हाल की कोई गतिविधि नहीं",
    geminiWaiting: "Gemini CLI chatminal workspace में auth का इंतज़ार कर रहा है",
  },
};
