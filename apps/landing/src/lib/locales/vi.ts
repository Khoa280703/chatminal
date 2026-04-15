import type { SiteDictionary } from "@/lib/site-dictionary";

import { githubReleasesUrl, latestReleaseTag, linuxDownloadUrl, windowsDownloadUrl } from "@/lib/landing-data";

export const viDictionary: SiteDictionary = {
  meta: {
    homeTitle: "Chatminal | Không gian terminal cho multi-agent coding",
    homeDescription:
      "Chatminal là terminal desktop để chạy nhiều phiên AI cùng lúc, gom chúng theo profile và quay lại đúng ngữ cảnh công việc thay vì bắt đầu lại từ đầu.",
    docsTitle: "Tài liệu Chatminal",
    docsDescription:
      "Tài liệu dành cho người dùng cuối về cài đặt, tổ chức và sử dụng Chatminal.",
  },
  header: {
    home: "Trang chủ",
    features: "Tính năng",
    downloads: "Tải về",
    docs: "Hướng dẫn",
    downloadCta: "Tải xuống",
    languageLabel: "Ngôn ngữ",
  },
  hero: {
    title: "Không gian terminal cho vibe coding.",
    description:
      "Chatminal giữ shell session, agent run và các setup lặp lại luôn gọn gàng, để bạn di chuyển giữa nhiều nhánh công việc song song mà không dồn hết vào một terminal.",
  },
  features: {
    items: [
      {
        icon: "robot_2",
        title: "Điều Khiển Nhiều Agent",
        description:
          "Chạy nhiều phiên AI song song, nhìn chúng trong cùng một cây phiên và chuyển qua lại giữa các nhánh mà không mất cấu trúc công việc.",
      },
      {
        icon: "integration_instructions",
        title: "Session Và Profile",
        description:
          "Nhóm session theo dự án, workflow hoặc team để từng shell context đứng riêng, thay vì biến thành một lịch sử terminal dài và lẫn lộn.",
      },
      {
        icon: "tune",
        title: "Quay Lại Việc Đang Làm Nhanh",
        description:
          "Giữ history của session, khôi phục layout làm việc và dùng startup command để mở lại những setup quen thuộc mà không cần dựng tay lại.",
      },
    ],
  },
  downloads: {
    title: "Tải về",
    description:
      "Chọn nền tảng rồi copy cách cài đặt phù hợp với cách bạn làm việc.",
    copiedLabel: "Đã copy",
    copyAndRunLabel: "Copy Và Chạy",
    terminalLabel: "cai-dat-terminal",
    platforms: [
      {
        id: "macos",
        label: "macOS",
        icon: "apple",
        artifact: `${latestReleaseTag} APPLE SILICON / INTEL`,
        downloadHref: githubReleasesUrl,
        directDownload: false,
        downloadLabel: "Mở Releases",
        helperText: "Chọn bản Apple Silicon hoặc Intel trên GitHub Releases.",
        methods: [
          {
            id: "brew",
            label: "brew",
            description: "Cài bằng Homebrew như một cask thông thường.",
            code: "brew install --cask chatminal",
          },
          {
            id: "bash",
            label: "bash",
            description: "Cài bản stable mới nhất bằng script cài đặt.",
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
        downloadLabel: "Tải Tarball",
        helperText: "Tải trực tiếp cho Linux x86_64.",
        methods: [
          {
            id: "bash",
            label: "bash",
            description: "Cài bản stable mới nhất bằng script cài đặt.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "tarball",
            label: "tarball",
            description: "Tải artifact Linux trực tiếp từ release.",
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
        downloadLabel: "Tải Zip",
        helperText: "Tải trực tiếp cho Windows x64.",
        methods: [
          {
            id: "powershell",
            label: "powershell",
            description: "Mở trang release mới nhất từ PowerShell.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
    ],
  },
  footer: {
    copyright: "© 2026 Bảo lưu mọi quyền.",
    home: "Trang chủ",
    userDocs: "Hướng dẫn dùng",
    githubRepo: "GitHub Repo",
    statusLog: "Nhật ký phát hành",
    devDocs: "Tài liệu dev",
  },
  docs: {
    sidebarTitle: "Trong trang này",
    eyebrow: "Hướng dẫn người dùng Chatminal",
    title: "Dùng Chatminal như một workspace để quay lại làm tiếp, không phải terminal dùng rồi bỏ.",
    description:
      "Trang này dành cho người dùng cuối, không phải contributor. Nó giải thích cách cài Chatminal, cách session và profile hoạt động cùng nhau, layout vận hành ra sao và bạn sẽ thấy gì khi quay lại công việc sau đó.",
    sections: [
      {
        id: "install",
        label: "Cài đặt",
        title: "Cài Chatminal",
        body:
          "Chatminal là terminal desktop cho những ai muốn session shell luôn có tổ chức và dễ tiếp tục lại. Hãy chọn cách cài phù hợp với nền tảng và cách bạn quản lý cập nhật.",
        bullets: [
          "Dùng script cài đặt nếu bạn muốn cách nhanh nhất trên macOS hoặc Linux.",
          "Dùng Homebrew trên macOS nếu bạn muốn cài và nâng cấp qua brew.",
          "Dùng GitHub Release nếu bạn thích tải trực tiếp gói ứng dụng hoặc đang ở Windows.",
          "Bản build sẵn hiện có cho macOS, Linux x86_64 và Windows x64.",
        ],
        methods: [
          {
            id: "bash",
            label: "bash",
            title: "Cài bằng Bash",
            body: "Cách này cài bản stable mới nhất và là đường ngắn nhất từ terminal.",
            code: "curl -fsSL https://chatminal.com/install | bash",
          },
          {
            id: "brew",
            label: "brew",
            title: "Cài bằng Homebrew",
            body: "Dùng trên macOS nếu bạn muốn Chatminal được quản lý như một cask bình thường.",
            code: "brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal\nbrew install --cask chatminal",
          },
          {
            id: "windows",
            label: "windows",
            title: "Tải cho Windows",
            body: "Hiện Windows được phát hành qua file zip ở GitHub Release mới nhất.",
            code: "start https://github.com/Khoa280703/chatminal/releases/latest",
          },
        ],
      },
      {
        id: "first-launch",
        label: "Lần đầu mở",
        title: "Điều gì xảy ra khi mở lần đầu",
        body:
          "Khi Chatminal mở lần đầu, nó khởi chạy shell mặc định của bạn và cho bạn một nơi sạch sẽ để tạo session phục vụ công việc thật, thay vì dồn hết vào một tab terminal duy nhất.",
        bullets: [
          "Bắt đầu với một shell session mới và kiểm tra môi trường mặc định của bạn.",
          "Tạo session riêng cho từng task hoặc repository.",
          "Dùng profile để nhóm các session liên quan lại với nhau.",
        ],
      },
      {
        id: "sessions-profiles",
        label: "Session",
        title: "Session và profile",
        body:
          "Session là đơn vị cốt lõi trong Chatminal. Mỗi session giữ shell, thư mục làm việc và trạng thái hoạt động riêng. Profile giúp bạn gom session theo dự án, team hoặc workflow.",
        bullets: [
          "Tạo một session cho mỗi task, repo hoặc môi trường.",
          "Chuyển profile khi bạn muốn đổi context mà không mất session hiện tại.",
          "Đặt lại tên session để sidebar phản ánh đúng việc nó đang làm.",
          "Dùng startup command cho những session bạn mở lại thường xuyên.",
        ],
      },
      {
        id: "layouts",
        label: "Layout",
        title: "Split và layout",
        body:
          "Chatminal được thiết kế cho việc làm trên nhiều session cùng lúc. Layout cho phép bạn chia workspace, giữ các session quan trọng luôn nhìn thấy và quay lại đúng bố cục quen thuộc sau này.",
        bullets: [
          "Chia workspace khi bạn cần xem log, shell output và một task khác cùng lúc.",
          "Dùng layout để giữ công việc chạy lâu luôn trong tầm mắt thay vì chuyển qua lại liên tục.",
          "Layout đã lưu giúp bạn mở lại đúng hình dạng workspace dễ hơn.",
        ],
      },
      {
        id: "history",
        label: "History",
        title: "History và khả năng resume",
        body:
          "Chatminal lưu trạng thái session để bạn quay lại làm việc mà không phải bắt đầu từ một terminal trắng mỗi lần. Điều đó bao gồm history của session và cả cấu trúc workspace.",
        bullets: [
          "History của session có thể được giữ lại để bạn vẫn xem được output cũ khi quay lại.",
          "Mở lại ứng dụng nên có cảm giác tiếp tục việc đang làm, không phải khởi động lại từ số không.",
          "Nếu muốn làm mới hoàn toàn, bạn có thể xóa history và reset context của session.",
        ],
      },
      {
        id: "startup-commands",
        label: "Khởi động",
        title: "Startup command",
        body:
          "Nếu một session luôn bắt đầu theo cùng một cách, hãy lưu startup command. Nó hữu ích khi cần mở dự án, attach vào tool hoặc khôi phục nhanh một flow shell quen thuộc.",
        bullets: [
          "Dùng startup command cho những session lặp lại mỗi ngày.",
          "Giữ chúng tập trung vào việc đưa bạn trở lại trạng thái làm việc nhanh nhất.",
          "Xem đây là tiện ích mở nhanh, không phải script triển khai đầy đủ.",
        ],
      },
      {
        id: "faq",
        label: "FAQ",
        title: "Câu hỏi thường gặp",
        body:
          "Hướng đi hiện tại của sản phẩm là desktop-first và xoay quanh session. Nếu bạn đang cân nhắc Chatminal có hợp workflow của mình không, đây là những câu hỏi quan trọng nhất.",
        bullets: [
          "Có hỗ trợ nhiều session không? Có, đó là phần cốt lõi của sản phẩm.",
          "Có thể tổ chức công việc theo profile không? Có, profile là một phần của mô hình workspace được lưu lại.",
          "Có nhớ layout và history không? Có, persistence nằm sẵn trong runtime và store.",
          "Trang này có dành cho contributor không? Không. Trang này viết cho người dùng cuối, không phải cho người đang hack vào repo.",
        ],
      },
    ],
  },
  preview: {
    welcomeBack: "Chào mừng quay lại Chatminal",
    tipsTitle: "Mẹo bắt đầu nhanh",
    tipsBody: "Chạy /init để tạo file CLAUDE.md chứa hướng dẫn cho workspace này.",
    recentTitle: "Hoạt động gần đây",
    recentEmpty: "Chưa có hoạt động gần đây",
    geminiWaiting: "Gemini CLI đang chờ xác thực trong workspace chatminal",
  },
};
