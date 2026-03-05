# Homebrew formula for Warden
# Place this file in your homebrew-core fork or use `brew tap YOUR_ORG/warden`
#
# Installation:
#   brew install YOUR_ORG/warden/warden
# or
#   brew tap YOUR_ORG/warden
#   brew install warden

class Warden < Formula
  desc "Historical code quality analysis and predictive architecture insights"
  homepage "https://github.com/YOUR_GITHUB_REPO"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/YOUR_GITHUB_REPO/releases/download/v0.1.0/warden-macos-aarch64.tar.gz"
      sha256 "YOUR_SHA256_ARM64"
    else
      url "https://github.com/YOUR_GITHUB_REPO/releases/download/v0.1.0/warden-macos-x86_64.tar.gz"
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
