# Changesets

本项目使用 pnpm 和 Changesets 管理 Rust 应用版本，不发布 npm 包。

```sh
pnpm install --frozen-lockfile
pnpm changeset
```

选择 `ddns-rust` 和 bump 类型，描述对用户的影响，随代码一起提交生成的 Markdown。
当前 0.x 版本中，不兼容变更使用 minor；兼容修复使用 patch。

合并到 main 后，Changesets 工作流自动创建或更新版本 PR。该 PR 会更新
package.json、Cargo.toml、Cargo.lock、CHANGELOG.md，并删除已消费的 changeset。
合并版本 PR 后自动创建 `v<version>` 标签，发布二进制和 GHCR 镜像。

完整配置和失败重试说明见仓库 README 的“开发与发布”章节。
