class Boxy < Formula
  desc "Unified package manager CLI and TUI"
  homepage "https://github.com/ljiulong/boxyy"
  version "0.6.3"
  url "https://github.com/ljiulong/boxyy/releases/download/v#{version}/boxy-cli-tui-v#{version}-macOS.tar.gz"
  sha256 "9434307d530464ad5ce276e063491bfc8ec97442f5976405e7baf7483c37630f"

  depends_on macos: :monterey

  def install
    bin.install "boxy-cli-macOS" => "boxy"
    bin.install "boxy-tui-macOS" => "boxy-tui"
  end

  test do
    system "#{bin}/boxy", "--version"
  end
end
