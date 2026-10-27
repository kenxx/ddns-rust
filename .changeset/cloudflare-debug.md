---
"ddns-rust": patch
---

完善 Cloudflare v4 DNS API 集成及 DEBUG 日志。

- 统一记录所有 Cloudflare API 请求和响应，包括方法、URL、请求头、请求体、响应状态码、响应头、响应体和耗时；请求 Authorization Token 脱敏。
- 解析响应前记录完整响应体，HTTP 错误、非 JSON 响应和网络异常也有 DEBUG 诊断信息。
- 使用 PATCH 仅更新已有记录的 IP，保留 TTL、代理状态等其他设置。
- 使用编码后的 `name.exact` 查询参数精确查找 A 记录。
- 增加连接和请求总超时，并保留 `result` 为空或缺失时的 Cloudflare 错误代码和消息。
