class Boxy < Formula
  desc "Unified package manager CLI and TUI"
  homepage "https://github.com/ljiulong/boxyy"
  version "1.2.0"
  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/boxy-cli-tui-v#{version}-macOS.tar.gz"
  sha256 "4f0bd0b10de8fb44663a13a1eb6e27e6d50a89083875cc4f7a98f72f5de346fe"

  depends_on macos: :monterey

  def install
    bin.install "boxy-cli-macOS" => "boxy"
    bin.install "boxy-tui-macOS" => "boxy-tui"
  end

  test do
    system "#{bin}/boxy", "--version"
  end
end
