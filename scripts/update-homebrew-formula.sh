#!/bin/sh
# Update the Homebrew formula with version + checksums from a GitHub Release.
# Usage: scripts/update-homebrew-formula.sh v0.1.0
set -eu

if [ $# -ne 1 ]; then
    echo "usage: $0 <version-tag>   (e.g. v0.1.0)" >&2
    exit 2
fi
TAG="$1"
VERSION="${TAG#v}"
REPO="zhiyue/jira-cli"
FORMULA="dist/homebrew/jira-cli.rb"

fetch_sha() {
    target="$1"
    url="https://github.com/${REPO}/releases/download/${TAG}/jira-cli-${TAG}-${target}.sha256"
    curl -fsSL "$url" | awk '{print $1}'
}

mac_arm="$(fetch_sha aarch64-apple-darwin)"
mac_x86="$(fetch_sha x86_64-apple-darwin)"
linux_arm="$(fetch_sha aarch64-unknown-linux-gnu)"
linux_x86="$(fetch_sha x86_64-unknown-linux-gnu)"

tmp="$(mktemp)"
sed \
  -e "s/version \"[^\"]*\"/version \"${VERSION}\"/" \
  -e "s/REPLACE_AARCH64_APPLE_DARWIN_SHA256/${mac_arm}/" \
  -e "s/REPLACE_X86_64_APPLE_DARWIN_SHA256/${mac_x86}/" \
  -e "s/REPLACE_AARCH64_UNKNOWN_LINUX_GNU_SHA256/${linux_arm}/" \
  -e "s/REPLACE_X86_64_UNKNOWN_LINUX_GNU_SHA256/${linux_x86}/" \
  "$FORMULA" > "$tmp"
mv "$tmp" "$FORMULA"

echo "Updated $FORMULA for $TAG."
echo ""
echo "Next steps:"
echo "  1. Review: git diff $FORMULA"
echo "  2. Copy to your tap repo:"
echo "     cp $FORMULA ../homebrew-tap/Formula/jira-cli.rb"
echo "  3. Commit and push in the tap repo."
echo ""
echo "Or publish it via a GitHub Action in the tap repo."
