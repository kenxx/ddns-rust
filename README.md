# DDNS Rust

一个简单的动态 DNS (DDNS) 服务，使用 Rust 编写，支持多个 DNS 提供商。

## 功能

- 支持通过 HTTP API 更新 DNS A 记录
- 支持多个 DNS 提供商配置
- 自动创建或更新记录
- 配置文件使用 TOML 格式

## 支持的 DNS 提供商

- [x] Cloudflare

## 安装

```bash
# 克隆项目
git clone <repository-url>
cd ddns-rust

# 编译
cargo build --release
```

## 配置

1. 复制示例配置文件:

```bash
cp config.example.toml config.toml
```

2. 编辑 `config.toml`，填入你的 DNS 提供商信息:

```toml
[server]
host = "0.0.0.0"
port = 3000

[[providers]]
name = "cloudflare"
type = "cloudflare"
auth = "basic"
username = "your_username"
password = "your_password"
api_key = "your_cloudflare_api_token"
zone_id = "your_zone_id"
```

每个 provider 必须配置 `auth`，仅支持以下两种鉴权方式，不能匿名更新：

- `auth = "basic"`：必须填写非空的 `username` 和 `password`，通过 HTTP Basic
  Authorization 头认证。密码填写原始密码，不能填写 H3C 的 `password cipher` 密文；
  用户名不能包含冒号，密码可以包含冒号。此模式不需要 `key`，即使配置了也不参与认证。
- `auth = "key"`：必须填写非空的 `key`，通过 URL 查询参数 `?key=...` 认证。
  此模式不会使用 Basic 认证头。

旧配置需要补上 `auth = "key"`（并填写 `key`），或改用上述 Basic 配置。
缺少 `auth`、选择不支持的鉴权方式、缺少所选方式要求的凭据时，服务启动失败。

### Cloudflare 配置说明

- `api_key`: Cloudflare API Token
  - 创建 API Token: Cloudflare Dashboard → My Profile → API Tokens → Create Token
  - Token 需要 `Zone.DNS` 的编辑权限
- `zone_id`: 在 Cloudflare Dashboard 中选择域名后，在右侧可以看到 Zone ID

## 运行

```bash
# 使用默认配置文件 (config.toml)
./target/release/ddns-rust

# 指定配置文件路径
./target/release/ddns-rust -c /path/to/config.toml

# 开启调试日志
RUST_LOG=debug ./target/release/ddns-rust
```

也可以在 `[server]` 中设置 `log_level = "debug"`。DEBUG 日志会输出请求方法、
完整 URI、HTTP 版本、全部请求头和完整请求体（文本及原始字节），包括
`Authorization`、Cookie 和 URL 中的密钥，仅在排查问题时开启。
请求体会在 DEBUG 模式下完整缓冲后交给处理器，排查结束后请恢复 `info`。
`RUST_LOG` 环境变量优先于配置文件。

Cloudflare API 调用也会在 DEBUG 级别记录请求方法、URL、请求头和 JSON 请求体，
以及响应状态码、响应头、完整响应体和耗时。发往 Cloudflare 的 Authorization
Token 会脱敏；HTTP 错误和非 JSON 响应也会在解析前记录，便于定位上游故障。
连接超时为 10 秒，每次 API 请求的总超时为 30 秒。

已有 DNS 记录使用 PATCH 仅更新 IP，保留 TTL、代理状态等其他设置；
新建 A 记录仍使用自动 TTL 和 `proxied = false`。

## API 使用

### 更新 DNS 记录

```
GET /ddns/{provider}/{host}/{ip}
```

**参数说明:**
- `provider`: 配置文件中定义的提供商名称
- `host`: 完整的主机名 (例如: `home.example.com`)
- `ip`: IPv4 地址

**示例:**

```bash
# Basic 认证（curl 会提示输入密码）
curl --user your_username "http://localhost:3000/ddns/cloudflare/home.example.com/1.2.3.4"

# auth = "key" 时
curl "http://localhost:3000/ddns/cloudflare/home.example.com/1.2.3.4?key=your_secret_key"
```

Basic 认证缺少或提供错误凭据时，返回 `401` 和
`WWW-Authenticate: Basic realm="ddns"`。H3C 可以继续使用
`/ddns/cloudflare/<h>/<a>`，配合 DDNS 策略中的 `username` 和 `password`。

**成功响应:**

```json
{
  "success": true,
  "message": "Updated record home.example.com to IP 1.2.3.4",
  "record_id": "abc123..."
}
```

**错误响应:**

```json
{
  "success": false,
  "error": "Provider not found: unknown"
}
```

### 健康检查

```bash
curl "http://localhost:3000/health"
```

## 在路由器/客户端上使用

可以在路由器或客户端上设置定时任务来自动更新 IP:

```bash
# 使用 curl 获取当前公网 IP 并更新
curl --user your_username "http://your-ddns-server:3000/ddns/cloudflare/home.example.com/$(curl -s ifconfig.me)"
```

## 开发与发布

需要 Rust、Node.js 22+ 和 pnpm 10.33.0（版本固定在 `packageManager`）。
可通过 Corepack 安装对应的 pnpm：`corepack enable && corepack install`。

```bash
pnpm install --frozen-lockfile
pnpm changeset            # 为本次改动选择 bump 类型并写变更说明
pnpm changeset status     # 查看待发布版本计划
pnpm check:versions       # 校验 package.json、Cargo.toml 和 Cargo.lock 一致
pnpm check:release        # 预演待发布变更；版本 PR 则校验现有版本及 CHANGELOG
pnpm test:release         # 测试版本同步和发布重试逻辑
```

发布流程：

1. 功能 PR 连同 changeset 合并到 `main`。
2. `Changesets` 工作流自动创建或更新版本 PR，生成 `CHANGELOG.md`，同步三个版本文件。
3. 审核并合并版本 PR 后，同一个 Changesets 工作流创建 `v<version>` 标签并发布。
4. 发布 Linux x86_64/ARM64、macOS ARM64、Windows x86_64 压缩包，以及 GHCR
   的 amd64/arm64 镜像和版本、`latest` 标签。

`package.json` 标记为 private，只用于发布工具和版本管理，不发布到 npm；不需要 NPM_TOKEN。
本地预演版本更新可运行 `pnpm version-packages`，它会消费 changeset 并修改版本文件，
日常开发只需提交 changeset，版本更新交给自动版本 PR。

仓库需要在 Settings → Actions → General → Workflow permissions 中允许
GitHub Actions 创建 Pull Request。默认使用 `GITHUB_TOKEN`。
如果分支保护要求自动版本 PR 上的 CI 检查，可配置 `RELEASE_TOKEN`（PAT 或 GitHub App
安装令牌，需要仓库 contents 和 pull requests 写权限）；默认机器人令牌创建的 PR
不会自动触发其他工作流。也可以通过 CI 的手动运行入口选择版本 PR 分支进行检查。

CI 会检查 Rust 格式、Clippy、测试、版本一致性、Changesets 状态、实际版本更新预演，
并验证 Docker 构建。版本 PR 合并后直接运行同一工作流中的构建和发布任务，
不依赖机器人推送标签触发新的工作流，也没有独立的 Release 工作流。

发布失败时，在 Actions 中打开对应的 Changesets 运行记录，重试失败任务或重新运行
全部任务；同一发布提交会复用已有标签。同一版本不会移动已有标签，普通后续提交
不会再次发布旧版本。手动推送标签不再触发发布。

## License

MIT

