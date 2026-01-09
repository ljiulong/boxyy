class Boxy < Formula
  desc "Unified package manager CLI and TUI"
  homepage "https://github.com/ljiulong/boxyy"
  version "1.0.0"
  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/boxy-cli-tui-v#{version}-macOS.tar.gz"
  sha256 "e95febbf409d25a1e6b916ddfc4274536a35f9c57679d535cb42d948a0ed33d5"

  depends_on macos: :monterey

  def install
    bin.install "boxy-cli-macOS" => "boxy"
    bin.install "boxy-tui-macOS" => "boxy-tui"
  end

  test do
    system "#{bin}/boxy", "--version"
  end
end
