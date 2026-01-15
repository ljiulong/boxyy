cask "boxy-gui" do
  version "1.2.0"
  sha256 "8989b09d06741a16bea8228fe83889d67ab99c7095ef977864cdadefd3eeb367"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
