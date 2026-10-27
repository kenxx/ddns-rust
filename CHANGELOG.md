# ddns-rust

## 0.2.0

### Minor Changes

- 5090762: 新增 HTTP Basic 认证，支持 H3C 路由器通过 DDNS 策略中的用户名和密码更新 DNS，无需在 URL 中携带 `?key=`。
  
  - 每个 provider 必须显式配置 `auth = "basic"` 或 `auth = "key"`。
  - Basic 模式必须配置非空的 `username` 和 `password`；认证失败返回 HTTP 401，并附带 `WWW-Authenticate: Basic realm="ddns"`。
  - Key 模式必须配置非空的 `key`，继续通过 `?key=...` 认证。两种模式仅接受各自的凭据。
  - 启动时拒绝缺少鉴权方式、缺少所需凭据、空凭据或 Basic 用户名包含冒号的配置。
  - DEBUG 日志新增完整请求信息，包括请求头和请求体，便于排查路由器鉴权问题。日志包含凭据，仅在排查时开启。
  - 更新配置示例和使用文档，补充鉴权测试。
  
  **配置迁移（不兼容变更）：** 原配置必须补充 `auth`。继续使用 query key 的用户需增加 `auth = "key"` 并保留非空 `key`；使用 Basic 的用户需配置 `auth = "basic"`、`username` 和原始 `password`。原先不配置 `key` 的匿名更新方式不再支持。

### Patch Changes

- db1bfed: 完善 Cloudflare v4 DNS API 集成及 DEBUG 日志。
  
  - 统一记录所有 Cloudflare API 请求和响应，包括方法、URL、请求头、请求体、响应状态码、响应头、响应体和耗时；请求 Authorization Token 脱敏。
  - 解析响应前记录完整响应体，HTTP 错误、非 JSON 响应和网络异常也有 DEBUG 诊断信息。
  - 使用 PATCH 仅更新已有记录的 IP，保留 TTL、代理状态等其他设置。
  - 使用编码后的 `name.exact` 查询参数精确查找 A 记录。
  - 增加连接和请求总超时，并保留 `result` 为空或缺失时的 Cloudflare 错误代码和消息。
- 9e8bcd6: 接入 pnpm 和 Changesets 完整发布流程：自动创建版本 PR、生成 CHANGELOG、同步 package.json 与 Cargo 版本，版本 PR 合并后创建 v 标签并发布多平台二进制和 Docker 镜像。增加版本同步、发布幂等性、Rust 检查、版本预演和 Docker 构建 CI。
- c3cc197: 修复自动版本 PR 已消费完 changeset 后，CI 仍调用 Changesets 3 的 version 命令而失败的问题。没有待发布变更时改为校验三个版本文件和对应 CHANGELOG；有变更时继续在临时目录预演升级，并覆盖这两种场景的回归测试。
