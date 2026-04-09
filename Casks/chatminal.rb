cask "chatminal" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.1.2"
  sha256 arm:   "8d1f2b884d06d28a3a40319b2a97fbe9a79cb5640c93eb72e23ee989fdc576b7",
         intel: "f1217ab420e0fd1b234d29585f00f1f3ba87a19bf3a8fba1fbc4fbcc6bdb2871"

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
