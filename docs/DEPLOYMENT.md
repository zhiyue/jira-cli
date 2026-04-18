# Deployment guide

Paths for distributing `jira-cli` to an internal team.

## 1. GitHub-based (simplest)

Once the maintainer tags `v0.1.0` and pushes, the release workflow publishes
binaries to the [Releases page](https://github.com/zhiyue/jira-cli/releases).
Team members then run the install script:

```bash
curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
```

## 2. Internal mirror (no GitHub access)

If the team machines can't reach github.com, host the release tarballs on an
internal HTTP server (nginx, Artifactory, S3 + presigned URLs):

```
https://internal-mirror.example.com/jira-cli/
├── v0.1.0/
│   ├── jira-cli-v0.1.0-x86_64-apple-darwin.tar.gz
│   ├── jira-cli-v0.1.0-x86_64-apple-darwin.tar.gz.sha256
│   ├── jira-cli-v0.1.0-aarch64-apple-darwin.tar.gz
│   ├── ... etc
```

Team members install with:

```bash
curl -sSL https://internal-mirror.example.com/install.sh | \
    sh -s -- -b https://internal-mirror.example.com/jira-cli
```

or via the env var approach:

```bash
export JIRA_CLI_DOWNLOAD_URL=https://internal-mirror.example.com/jira-cli
curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
```

## 3. Homebrew tap (for macOS-heavy teams)

Publish a tap repo at `github.com/<org>/homebrew-jira-cli` containing:

```
Formula/
  jira-cli.rb
```

Populate `jira-cli.rb` by running `scripts/update-homebrew-formula.sh v0.1.0`
after the release workflow completes. Team members:

```bash
brew tap <org>/jira-cli
brew install jira-cli
```

## 4. Shared config via dotfiles / puppet / ansible

Roll out a shared `~/.config/jira-cli/config.toml` with the team's Jira URL +
commonly-used `[field_renames]` and `[jql_aliases]`. Each user still has to
supply their own `user` + `password`, either by editing the file or by env vars.

Example ansible task:

```yaml
- name: Install jira-cli
  ansible.builtin.shell: |
    curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | \
      sh -s -- -v v0.1.0 -d /usr/local/bin
  args:
    creates: /usr/local/bin/jira-cli

- name: Distribute shared config template
  ansible.builtin.template:
    src: templates/jira-cli-config.toml.j2
    dest: "{{ ansible_user_dir }}/.config/jira-cli/config.toml"
    mode: '0600'
```

Template would have `url`, `[field_renames]`, `[jql_aliases]`, `[defaults]` —
but leave `user` / `password` blank (each engineer fills those in, or has them
come from env / keychain).

## 5. Docker image (for CI)

The release workflow does not currently build a container image. If needed,
add a Dockerfile:

```dockerfile
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY jira-cli /usr/local/bin/jira-cli
ENTRYPOINT ["/usr/local/bin/jira-cli"]
```

Build locally after downloading the `x86_64-unknown-linux-musl` tarball:

```bash
curl -L https://github.com/zhiyue/jira-cli/releases/download/v0.1.0/jira-cli-v0.1.0-x86_64-unknown-linux-musl.tar.gz | tar -xz
docker build -t jira-cli:v0.1.0 .
```
