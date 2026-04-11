cask "chatminal" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.1.3"
  sha256 arm:   "e7e2a0062940a3d9afb8b489f18c002f2b32eb52a2a1f2e771a696d2dd4e528d",
         intel: "40f25a46ec29debe578894921e9f68ed1f572a3ba311f1033d720ed19f74a143"

  url "https://github.com/Khoa280703/chatminal/releases/download/v#{version}/Chatminal-v#{version}-macos-#{arch}.dmg"
  name "Chatminal"
  desc "A modern terminal emulator"
  homepage "https://chatminal.com"

  app "Chatminal.app"
  binary "#{appdir}/Chatminal.app/Contents/MacOS/chatminal-desktop", target: "chatminal"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Chatminal.app"]
  end

  zap trash: [
    "~/Library/Application Support/chatminal",
    "~/Library/Caches/chatminal",
    "~/Library/Logs/chatminal",
    "~/.local/share/chatminal",
  ]
end
