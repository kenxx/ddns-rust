---
"ddns-rust": patch
---

接入 pnpm 和 Changesets 完整发布流程：自动创建版本 PR、生成 CHANGELOG、同步 package.json 与 Cargo 版本，版本 PR 合并后创建 v 标签并发布多平台二进制和 Docker 镜像。增加版本同步、发布幂等性、Rust 检查、版本预演和 Docker 构建 CI。
