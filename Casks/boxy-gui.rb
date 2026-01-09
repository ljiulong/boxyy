cask "boxy-gui" do
  version "0.6.4"
  sha256 "4227db97cbdb931a53c8949631ceea1d1ed1a95598a18528d46f4f67bde59806"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
