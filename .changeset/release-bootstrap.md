---
"ddns-rust": patch
---

修复发布前第二次 checkout 清理 node_modules 导致打标签脚本缺少 smol-toml 的问题，在干净检出后重新执行冻结依赖安装。增加隔离 Git 仓库中的清理、重装依赖、实际打标签与重复运行测试，并更新 Action 的 Node.js 运行时及固定 Ubuntu 版本。
