# frozen_string_literal: true

class JiraCli < Formula
  desc "Agent-first CLI for legacy Jira Server 8.13.5"
  homepage "https://github.com/zhiyue/jira-cli"
  license "MIT OR Apache-2.0"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/zhiyue/jira-cli/releases/download/v#{version}/jira-cli-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_AARCH64_APPLE_DARWIN_SHA256"
    else
      url "https://github.com/zhiyue/jira-cli/releases/download/v#{version}/jira-cli-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_X86_64_APPLE_DARWIN_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/zhiyue/jira-cli/releases/download/v#{version}/jira-cli-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_AARCH64_UNKNOWN_LINUX_GNU_SHA256"
    else
      url "https://github.com/zhiyue/jira-cli/releases/download/v#{version}/jira-cli-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_X86_64_UNKNOWN_LINUX_GNU_SHA256"
    end
  end

  def install
    bin.install "jira-cli"
  end

  test do
    assert_match "jira-cli", shell_output("#{bin}/jira-cli --version")
  end
end
