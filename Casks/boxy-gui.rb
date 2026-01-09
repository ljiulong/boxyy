cask "boxy-gui" do
  version "1.0.0"
  sha256 "b9a8e5ee21b4b7477f87fe4c0f2a06b258f82e086ea4e6debd61edbfaa382b50"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
