cask "boxy-gui" do
  version "1.1.0"
  sha256 "d9d54e4ec613ea091e46ebb791c7a53f50a145925babefae6063d5839bb555c0"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
