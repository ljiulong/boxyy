class Boxy < Formula
  desc "Unified package manager CLI and TUI"
  homepage "https://github.com/ljiulong/boxyy"
  version "0.6.4"
  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/boxy-cli-tui-v#{version}-macOS.tar.gz"
  sha256 "18936530cf7016ec39177d7bd6b07c839c67d9dbc225fb2f05e0a6878fa33c95"

  depends_on macos: :monterey

  def install
    bin.install "boxy-cli-macOS" => "boxy"
    bin.install "boxy-tui-macOS" => "boxy-tui"
  end

  test do
    system "#{bin}/boxy", "--version"
  end
end
