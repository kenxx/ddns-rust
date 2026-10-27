---
"ddns-rust": minor
---

新增 HTTP Basic 认证，支持 H3C 路由器通过 DDNS 策略中的用户名和密码更新 DNS，无需在 URL 中携带 `?key=`。

- 每个 provider 必须显式配置 `auth = "basic"` 或 `auth = "key"`。
- Basic 模式必须配置非空的 `username` 和 `password`；认证失败返回 HTTP 401，并附带 `WWW-Authenticate: Basic realm="ddns"`。
- Key 模式必须配置非空的 `key`，继续通过 `?key=...` 认证。两种模式仅接受各自的凭据。
- 启动时拒绝缺少鉴权方式、缺少所需凭据、空凭据或 Basic 用户名包含冒号的配置。
- DEBUG 日志新增完整请求信息，包括请求头和请求体，便于排查路由器鉴权问题。日志包含凭据，仅在排查时开启。
- 更新配置示例和使用文档，补充鉴权测试。

**配置迁移（不兼容变更）：** 原配置必须补充 `auth`。继续使用 query key 的用户需增加 `auth = "key"` 并保留非空 `key`；使用 Basic 的用户需配置 `auth = "basic"`、`username` 和原始 `password`。原先不配置 `key` 的匿名更新方式不再支持。
