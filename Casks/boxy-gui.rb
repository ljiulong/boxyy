cask "boxy-gui" do
  version "0.6.2"
  sha256 "6aec0a404eef1bbdfdccb061bca3fc2823ba768def73a2d7cc6cfb9694f242fd"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
