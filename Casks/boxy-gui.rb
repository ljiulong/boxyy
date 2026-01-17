cask "boxy-gui" do
  version "1.2.1"
  sha256 "78fd2d2d316f0368ee0a14dd9268b539c5c2e67eea64996e4565bbacbe5cbec9"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
