cask "boxy-gui" do
  version "0.5.6"
  sha256 "f9dc9a930ee1ccb818adef3c442ebcae24c14b048d54686bc5c30e43e8d132f7"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
