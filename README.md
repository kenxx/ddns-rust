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

[[provider]]
name = "cloudflare"
type = "cloudflare"
api_key = "your_cloudflare_api_token"
zone_id = "your_zone_id"
```

### Cloudflare 配置说明

- `api_key`: Cloudflare API Token（推荐）或 Global API Key
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
# 更新记录
curl "http://localhost:3000/ddns/cloudflare/home.example.com/1.2.3.4"
```

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
curl "http://your-ddns-server:3000/ddns/cloudflare/home.example.com/$(curl -s ifconfig.me)"
```

## License

MIT

