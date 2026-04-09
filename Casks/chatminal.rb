cask "chatminal" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.1.1"
  sha256 arm:   "5a0a4d9037377b5051c471fcfb8a066ded4e0e424b80ee9f1a2dfb7dd5f6ff3c",
         intel: "cf1a8121376442622667c262a294eb85c8751fba58331800aa3516ad54974049"

  url "https://github.com/Khoa280703/chatminal/releases/download/v#{version}/Chatminal-v#{version}-macos-#{arch}.dmg"
  name "Chatminal"
  desc "A modern terminal emulator"
  homepage "https://chatminal.com"

  app "Chatminal.app"
  binary "#{appdir}/Chatminal.app/Contents/MacOS/chatminal-desktop", target: "chatminal"

  zap trash: [
    "~/Library/Application Support/chatminal",
    "~/Library/Caches/chatminal",
    "~/Library/Logs/chatminal",
    "~/.local/share/chatminal",
  ]
end
