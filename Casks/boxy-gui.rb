cask "boxy-gui" do
  version "0.6.3"
  sha256 "ccf3d8b81140f1d22f0709278edc1f349c53a6e134944dd2e21cc42d58c62d69"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
