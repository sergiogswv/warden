# Homebrew formula for Warden
# Place this file in your homebrew-core fork or use `brew tap sergiogswv/warden`
#
# Installation:
#   brew install sergiogswv/warden/warden
# or
#   brew tap sergiogswv/warden
#   brew install warden

class Warden < Formula
  desc "Historical code quality analysis and predictive architecture insights"
  homepage "https://github.com/sergiogswv/warden"
  version "0.2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/sergiogswv/warden/releases/download/v0.2.0/warden-macos-aarch64.tar.gz"
      sha256 "YOUR_SHA256_ARM64"
    else
      url "https://github.com/sergiogswv/warden/releases/download/v0.2.0/warden-macos-x86_64.tar.gz"
      sha256 "YOUR_SHA256_X86_64"
    end
  end

  def install
    bin.install "warden"
  end

  test do
    system "#{bin}/warden", "--version"
  end
end
