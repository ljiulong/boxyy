cask "boxy-gui" do
  version "0.3.5"
  sha256 "053e71041629a23bcc8019c5b549aa40d77a8d19eab2c702fbd678e4b9f5146c"

  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/Boxy-v#{version}-macos.dmg"
  name "Boxy"
  desc "Unified package manager GUI"
  homepage "https://github.com/ljiulong/boxyy"

  app "Boxy.app"
end
