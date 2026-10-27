---
"ddns-rust": patch
---

修复自动版本 PR 已消费完 changeset 后，CI 仍调用 Changesets 3 的 version 命令而失败的问题。没有待发布变更时改为校验三个版本文件和对应 CHANGELOG；有变更时继续在临时目录预演升级，并覆盖这两种场景的回归测试。
