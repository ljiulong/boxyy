cask "boxy-gui" do
  version "0.3.0"
  sha256 "REPLACE_WITH_SHA256"

  arch arm: "aarch64", intel: "x64"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy_#{version}_#{arch}.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
