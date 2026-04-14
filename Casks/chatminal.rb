cask "chatminal" do
  arch arm: "aarch64", intel: "x86_64"

  version "0.1.5"
  sha256 arm:   "a8e0a7e47b345bfe8bea1ce1282f250e69c98f086a148901e2e0b19b77cb0a74",
         intel: "85c34ee4c9998a602851286cf320042cd93b595fa2f8f2f8da687b7efaf55698"

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
