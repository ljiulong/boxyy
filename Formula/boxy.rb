class Boxy < Formula
  desc "Unified package manager CLI and TUI"
  homepage "https://github.com/ljiulong/boxyy"
  version "0.3.0"
  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/boxy-cli-tui-v#{version}-macOS.tar.gz"
  sha256 "REPLACE_WITH_SHA256"

  depends_on macos: :monterey

  def install
    bin.install "boxy-cli-macOS" => "boxy"
    bin.install "boxy-tui-macOS" => "boxy-tui"
  end

  test do
    system "#{bin}/boxy", "--version"
  end
end
