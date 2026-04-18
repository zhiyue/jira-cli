#!/usr/bin/env bash
# Preflight probe for the jira-cli skill.
# Exit 0 if the binary is on PATH (echoing its --version line to stdout).
# Exit 1 with an install hint on stderr if it is not.
set -u

if command -v jira-cli >/dev/null 2>&1; then
    jira-cli --version
    exit 0
fi

cat >&2 <<'HINT'
jira-cli: not found on PATH.

Install one of:
  brew install zhiyue/tap/jira-cli
  curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh

Docs: https://github.com/zhiyue/jira-cli#install
HINT
exit 1
