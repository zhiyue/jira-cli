# AUA 2026-04 提名设计稿 — Agent-first CLI 双子星

- 活动：🥇 2026-04 Actually Useful Award 月度最佳 AI 实践活动
- 活动文档：https://deepwisdom.feishu.cn/wiki/GrrUw2eBuiOXyvknaf6cc0IQnKd
- 参考样例：https://deepwisdom.feishu.cn/docx/MCC1dgNA1o3kwLxdNXocH3a0nSe （BugBuster）
- 提名截止：2026-04-20 周一 18:00
- 赛道：**🏆 Best Quick Win 最佳效率提升**
- 叙事：方案 2（双子星：精准对齐冻结版）为主，结合方案 1（CLI vs MCP 进程经济学）做量化支撑

## 背景

公司生产环境长期冻结在 **Jira Server 8.13.5** 与 **GitLab EE 14.0.5**。市面主流 CLI（官方 `atlassian` / `glab` / `ankitpokhrel/jira-cli` 等）都以 Cloud 或 15.x+ 为默认目标,在这两个版本上要么启动失败、要么部分 API 直接返回 404。

Agent 在跨 Jira/GitLab 协作时,业界常见两种方案:

1. **MCP daemon**:每个并发 agent session 跑一个常驻进程(Node/Python, 30–80 MiB RSS),空闲也不释放。
2. **自写 REST wrapper**:每个 agent 团队重复发明,接口形状、错误处理、退出码都不统一。

这两种方案在"旧版冻结 Jira/GitLab"这个具体场景下,都有额外代价:旧版 API 的 quirks(如 `mr diffs` 在 15.7 才引入、Story Points 有 4 个同名 customfield id)会反复消耗 agent 的时间/tokens。

## 目标（Quick Win 口径）

- 给 8.13.5 与 14.0.5 补上唯一原生的 agent-first CLI,让 agent 以 `bash -c` + JSON 的最简方式访问这两个系统。
- 通过 fork-exec 短命进程 + JSON stdout 约定,把单次交互的内存/协议/冷启动成本压到最低。
- 把两把 CLI 的 agent-first 脚手架(JSON I/O 约定、`hint` 字段、exit code、schema/manifest 自描述)沉淀为可 fork 的方法论。

## 9 节提名模板内容

### 1. 名称 & 链接
- jira-cli — https://github.com/zhiyue/jira-cli · `brew install zhiyue/tap/jira-cli`
- gitlab-cli — https://github.com/zhiyue/gitlab-cli · `brew install zhiyue/tap/gitlab-cli`
- Homebrew Tap — https://github.com/zhiyue/homebrew-tap

### 2. 核心说明（优化了哪一步）
公司生产用的是冻结版 Jira Server 8.13.5 + GitLab EE 14.0.5。市面上主流 CLI 都假设 Cloud 或 15.x+,在这两个版本上要么用不起来、要么部分 API 直接 404。本作品补上这两个旧版**唯一原生的 agent-first CLI**,让 agent 用一行 `bash -c` 直读直写 Jira/GitLab,替代"自己写 REST wrapper"或"跑常驻 MCP daemon"。

### 3. 解决方式
- **精准对齐冻结版 API**:jira-cli 说 REST v2 + Agile 1.0(8.13.5 无 PAT,支持 Basic + cookie 会话);gitlab-cli 说 14.0.5 原生接口,`manifest` 内嵌 10+ 条 API quirks 对照(如 `mr diffs` 404 → `mr changes`)。
- **Agent-first 输出约定**:stdout 结构化 JSON/NDJSON,stderr 结构化 error JSON(带 `hint` 字段),稳定 exit codes `0/2/3/4/5/6/7/8/9/10`。
- **Schema 自描述**:`jira-cli schema` + `gitlab manifest`,agent 零文档冷启动、按需 drill-down。
- **选 CLI 而非 MCP**:fork-exec 短命进程,避开"每个 agent session 常驻 daemon"的内存税。
- **一键分发**:Rust 单二进制 → Homebrew tap / `curl | sh` / PowerShell / `cargo install` / Claude Code 插件 / Codex skill 全齐。

### 4. 使用场景（Where）
- 任何 Claude Code / Codex / Copilot CLI agent 会话里的 Jira/GitLab 调用节点(`bash -c` + JSON)。
- Sentry → Jira → GitLab 自动化流水线(如 BugBuster 类修复机器人,调 Jira/GitLab 那几步可直接换成这两把 CLI)。
- CI/CD 中 `jq` / `xargs` / cron 粘合命令。
- 人肉工程师顺手查 issue / MR。
- **使用频率**:每天(agent-first 工具链里的底层 plumbing 组件)。

### 5. 关键效果（Impact）
- **内存数量级下降**:单次调用峰值 **~4 MiB**(结束清零)vs Node/Python 版 MCP 常驻 **30–80 MiB/session**;5 个并发 agent session ≈ 省 **150–400 MiB** 常占。
- **零协议锁定**:JSON over stdout,任何 agent runtime 都能消费,host 不需要实现 MCP 协议。
- **零文档冷启动**:schema / manifest 让 agent 自发现能力,~3 KB 索引 + 按需 drill-down。
- **覆盖度**:gitlab-cli 17 个顶层命令覆盖 14.0.5 的 MR / issue / pipeline / job / commit / repo / file / label / note / discussion / search;jira-cli 全量覆盖 8.13.5 的 issue / search / agile / field 等高频域。
- **可组合**:pipe 到 `jq`、`xargs`、重定向、CI 变量、shell 脚本——agent 无需特殊适配。

### 6. Before / After

| 维度 | Before(MCP / 手搓 wrapper) | After(jira-cli + gitlab-cli) |
|---|---|---|
| 内存 | 30–80 MiB 常驻 × N sessions | ~4 MiB 峰值,结束归零 |
| 协议 | host 要实现 MCP 栈 | JSON over stdout,shell 即可消费 |
| 命令发现 | 读 README / API 文档 | `jira-cli schema` / `gitlab manifest` 一行 |
| 14.0.5 quirks | 调 `mr diffs` → 404 → 自己查 | 内嵌 quirks 表 + `hint` 字段直接告诉"改用 `mr changes`" |
| 冻结版覆盖 | 主流 CLI 默认 Cloud/15.x+,用不起来 | 精准对齐 8.13.5 / 14.0.5 |

### 7. 复用方式（Reuse）
```bash
# macOS / Linux
brew install zhiyue/tap/jira-cli
brew install zhiyue/tap/gitlab-cli

# Windows PowerShell
irm https://raw.githubusercontent.com/zhiyue/gitlab-cli/main/install.ps1 | iex

# Rust toolchain
cargo install --git https://github.com/zhiyue/jira-cli --locked

# Claude Code 插件
/plugin marketplace add zhiyue/jira-cli
/plugin install jira-cli@jira-cli
```

- 首次配置:`jira-cli config init` / `gitlab config set-token` 各一次
- 开源 Apache-2.0;公司团队如需给其他旧系统(Jenkins、Confluence、Bamboo 等)写 agent-first CLI,可 fork 这两份仓库当骨架(同套 Rust 脚手架 / JSON I/O / hint / exit code / schema 契约)。

### 8. 为什么值得提名（Why）
- 🏆 **Quick Win 硬指标**:一个数量级的内存节省(README bench 有实测数据支撑)。
- 🏆 **可复制**:别人今天 `brew install` 就能用;两份代码共享同一套 agent-first 脚手架,方法论沉淀为可 fork 的骨架。
- 🏆 **填真实空白**:8.13.5 + 14.0.5 是公司现实约束,业界没有精准对齐的 agent-first CLI,这两把是"唯一解"。
- 🏆 **方法论可被复用**:README 里的"为什么 CLI 而不是 MCP"论证(进程经济学 / 可组合 / 0 协议锁定)可直接作为团队 agent-first 工具选型的参考。

### 9. 贡献者
- 部门:深圳后台
- Owner:@戴智斌
- 协作者:无

## 发布流程

1. 将上面 9 节内容以 Markdown 形式作为飞书文档正文。
2. 用 `lark-cli docs +create` 创建文档,标题 `Agent-first CLI 双子星(AUA 2026-04 提名)`(≤ 27 字符兼容 docx import 限制,必要时截短)。
3. 如发布接口对单次 children 数量有限制(≤ 50),把内容拆成分批 `+update` 追加。
4. 回传文档 URL,提醒用户在活动文档 `# 提名 / 🏆 Best Quick Win` 节底下追加一行 `深圳后台:Agent-first CLI 双子星 @戴智斌` + 文档链接。

## 参考

- 活动文档 docx token: `LvpDdhxacorSdfxkUQ1c0Ccen1d`(wiki token `GrrUw2eBuiOXyvknaf6cc0IQnKd` 解出)
- 参考文档 docx token: `MCC1dgNA1o3kwLxdNXocH3a0nSe`
- jira-cli README: `/Users/zhiyue/workspace/jira-cli/README.md`
- gitlab-cli README: `/Users/zhiyue/workspace/gitlab-cli/README.md`
