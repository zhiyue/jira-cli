# Releasing jira-cli

SOP for cutting a new `vX.Y.Z` release. Aimed at the maintainer; no prior
release context required.

## 0. Prerequisites

- Working directory is clean and on `main` (`git status` empty).
- `origin/main` CI is green on the last commit.
- You've decided the bump type using semver:
  - **patch** (`0.2.1` → `0.2.2`) — bug fixes, docs, non-breaking internal tweaks.
  - **minor** (`0.2.x` → `0.3.0`) — new user-visible commands, flags, or config.
  - **major** — reserved for removed commands, renamed flags, breaking config.

The release workflow (`.github/workflows/release.yml`) is triggered by pushing
a `v*.*.*` tag. It runs the full distribution pipeline: GitHub Release,
cross-compiled binaries for 6 targets, sha256 checksums, and an auto-commit
that regenerates `dist/homebrew/jira-cli.rb` on `main`.

## 1. Prepare the release commit locally

Edit these files in one commit:

1. `Cargo.toml` — bump `version = "X.Y.Z"`.
2. `dist/plugin/.claude-plugin/plugin.json` — bump `version` to `X.Y.Z`.
3. `.claude-plugin/marketplace.json` — bump the `plugins[0].version`
   field to `X.Y.Z` to match.
4. `CHANGELOG.md` — insert a new section above the latest one:

   ```
   ## [X.Y.Z] - YYYY-MM-DD

   ### Added | Changed | Fixed | Removed
   - <user-facing summary; why it matters, not just what>
   ```

5. `Cargo.lock` — refresh it by running `cargo build` (don't edit by hand).

Verify the new version locally:

```bash
cargo build
./target/debug/jira-cli --version
# → jira-cli X.Y.Z (target=<triple>, git=<12-char-sha>)
cargo test
```

Commit with a message that describes the change *and* names the version.
Historical style (`git log --oneline`) uses one of:

```
feat(<scope>): <summary> and release X.Y.Z
fix(<scope>):  <summary> and release X.Y.Z
chore(release): X.Y.Z            # when the release is pure packaging
```

## 2. Sync with origin before pushing

After a successful release, the previous run's `bump-homebrew` job pushes
a `chore(homebrew): update formula for vX.Y.Z` commit directly to `main`.
If you haven't pulled since, your local `main` is behind:

```bash
git fetch origin
git log --oneline HEAD..origin/main   # expect 1 formula-update commit
git pull --rebase origin main
```

Because the rebase changes HEAD, rebuild once so the embedded git hash
in `--version` matches what you're about to ship:

```bash
cargo build && ./target/debug/jira-cli --version
```

## 3. Push main, tag, push tag

```bash
git push origin main
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

**This is the point of no return.** The tag push triggers CI which creates
a public GitHub Release and auto-commits back to `main`.

## 4. Watch CI to green

```bash
gh run watch --workflow release.yml                 # or:
gh run list  --workflow release.yml --limit 1
```

Three jobs must all succeed:

| Job              | What it does                                             |
| ---------------- | -------------------------------------------------------- |
| `create-release` | Publishes a GitHub Release from the CHANGELOG entry.     |
| `build` (×6)     | Cross-compiles binaries + sha256 for each target.        |
| `bump-homebrew`  | Regenerates `dist/homebrew/jira-cli.rb`, pushes to main, |
|                  | and (if tap secrets are set) mirrors to the tap repo.    |

Typical runtime: ~3 minutes.

### If `build` fails for one target

Binaries for the other targets are still published. Two options:
1. **Fix-forward**: investigate, bump to the next patch, go through steps 1–3
   again. This is the default — published tags should not be mutated.
2. Re-run just the failed matrix leg via `gh run rerun --failed <run-id>` if
   the failure was infra flake (runner queue starvation, network timeout).

Never delete a pushed tag that already has a GitHub Release, even if some
artifacts are missing; downstream installers may already be caching it.

## 5. Verify the published release

```bash
gh release view vX.Y.Z

# Binary install end-to-end:
curl -fsSL https://github.com/zhiyue/jira-cli/releases/download/vX.Y.Z/jira-cli-vX.Y.Z-aarch64-apple-darwin.tar.gz \
  | tar -xz -O jira-cli/jira-cli | head -c 0   # just checks the asset exists

# Homebrew tap picked up the bump (runs the formula's url/sha256 against GH):
brew update
brew reinstall zhiyue/tap/jira-cli
jira-cli --version
```

## 6. Post-release checklist

- [ ] `gh release view vX.Y.Z` shows all expected assets + `.sha256` files.
- [ ] `dist/homebrew/jira-cli.rb` on `main` has the new version + hashes.
- [ ] The auto-commit `chore(homebrew): update formula for vX.Y.Z` is present.
- [ ] `brew reinstall zhiyue/tap/jira-cli` yields the new `--version` string.
- [ ] Pull the auto-commit locally so the next release starts in sync:
      `git pull --rebase origin main`.

## Target matrix note

`x86_64-apple-darwin` is currently disabled in `release.yml` due to
`macos-13` runner starvation. Intel Mac users install the
`aarch64-apple-darwin` binary and it runs under Rosetta 2. When the
runner issue clears, re-enable that matrix entry and the matching
`fetch_sha` line in `scripts/update-homebrew-formula.sh` in the same
commit.

## Non-git build environments

The embedded git hash in `--version` comes from `build.rs` running
`git rev-parse --short=12 HEAD`. If the source is built from a tarball
(no `.git` directory), the hash falls back to `unknown`; this is
expected and not a release blocker. Homebrew and `cargo install` both
have git context during their builds.

## Skill / plugin-only hotfixes

When you need to ship a change that lives entirely under `dist/plugin/`
or in the root `.claude-plugin/marketplace.json` — for example, a
correction to `SKILL.md`, a new workflow in `references/workflows.md`,
or fixing `check.sh` — you do **not** need to cut a crate release.

Flow:

1. Make the change on a branch, land it on `main` via a normal commit.
2. Bump `dist/plugin/.claude-plugin/plugin.json#version` *and* the
   matching entry in `.claude-plugin/marketplace.json` to the next
   patch (e.g. `0.2.2` → `0.2.3`) even though `Cargo.toml` is still
   at `0.2.2`. This is the one situation where the plugin version
   runs ahead of the crate.
3. The next crate release realigns them by bumping `Cargo.toml` to
   the plugin's version (or higher) in the same release commit.

No tag is required for a plugin-only hotfix. Users on Claude Code
pick it up via `/plugin update jira-cli`; Codex users via
`git pull` + `codex` restart.
