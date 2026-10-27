use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{debug, info};
use reqwest::{header, Client, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::DnsUpdateResult;
use crate::config::ProviderConfig;

const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";

pub async fn update_record(
    config: &ProviderConfig,
    host: &str,
    ip: &str,
) -> Result<DnsUpdateResult> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create Cloudflare HTTP client")?;

    if let Some(existing) = get_record(&client, config, host).await? {
        if existing.content == ip {
            info!("Record {} already has IP {}, no update needed", host, ip);
            return Ok(DnsUpdateResult {
                success: true,
                message: format!("Record already up to date with IP {}", ip),
                record_id: Some(existing.id),
            });
        }

        info!(
            "Updating existing record {} from {} to {}",
            host, existing.content, ip
        );
        let record = update_existing_record(&client, config, &existing.id, ip).await?;
        Ok(DnsUpdateResult {
            success: true,
            message: format!("Updated record {} to IP {}", host, ip),
            record_id: Some(record.id),
        })
    } else {
        info!("Creating new record {} with IP {}", host, ip);
        let record = create_record(&client, config, host, ip).await?;
        Ok(DnsUpdateResult {
            success: true,
            message: format!("Created new record {} with IP {}", host, ip),
            record_id: Some(record.id),
        })
    }
}

async fn get_record(
    client: &Client,
    config: &ProviderConfig,
    host: &str,
) -> Result<Option<DnsRecord>> {
    let records: Vec<DnsRecord> = send_request(client, list_request(client, config, host)).await?;
    Ok(records.into_iter().next())
}

fn list_request(client: &Client, config: &ProviderConfig, host: &str) -> RequestBuilder {
    client
        .get(format!(
            "{}/zones/{}/dns_records",
            CLOUDFLARE_API_BASE, config.zone_id
        ))
        .bearer_auth(&config.api_key)
        .query(&[("type", "A"), ("name.exact", host)])
}

async fn create_record(
    client: &Client,
    config: &ProviderConfig,
    host: &str,
    ip: &str,
) -> Result<DnsRecord> {
    let body = CreateRecordRequest {
        record_type: "A",
        name: host,
        content: ip,
        ttl: 1,
        proxied: false,
    };
    send_request(
        client,
        client
            .post(format!(
                "{}/zones/{}/dns_records",
                CLOUDFLARE_API_BASE, config.zone_id
            ))
            .bearer_auth(&config.api_key)
            .json(&body),
    )
    .await
}

async fn update_existing_record(
    client: &Client,
    config: &ProviderConfig,
    record_id: &str,
    ip: &str,
) -> Result<DnsRecord> {
    send_request(client, update_request(client, config, record_id, ip)).await
}

fn update_request(
    client: &Client,
    config: &ProviderConfig,
    record_id: &str,
    ip: &str,
) -> RequestBuilder {
    client
        .patch(format!(
            "{}/zones/{}/dns_records/{}",
            CLOUDFLARE_API_BASE, config.zone_id, record_id
        ))
        .bearer_auth(&config.api_key)
        // Preserve TTL, proxy status, tags, and all other existing record settings.
        .json(&serde_json::json!({ "content": ip }))
}

async fn send_request<T: DeserializeOwned>(client: &Client, builder: RequestBuilder) -> Result<T> {
    let request = builder
        .build()
        .context("Failed to build Cloudflare request")?;
    let method = request.method().clone();
    let url = request.url().clone();
    let start = Instant::now();
    if log::log_enabled!(log::Level::Debug) {
        let mut headers = request.headers().clone();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_static("Bearer [REDACTED]"),
        );
        let body = request
            .body()
            .and_then(|body| body.as_bytes())
            .unwrap_or_default();
        debug!(
            "Cloudflare req: method={} url={} version={:?} headers={:?} body={:?}",
            method,
            url,
            request.version(),
            headers,
            String::from_utf8_lossy(body)
        );
    }

    let response = match client.execute(request).await {
        Ok(response) => response,
        Err(error) => {
            debug!(
                "Cloudflare req failed: method={} url={} elapsed_ms={} error={:?}",
                method,
                url,
                start.elapsed().as_millis(),
                error
            );
            return Err(error).context("Failed to send request to Cloudflare");
        }
    };
    let status = response.status();
    debug!(
        "Cloudflare resp: method={} url={} version={:?} status={} headers={:?}",
        method,
        url,
        response.version(),
        status,
        response.headers()
    );
    let body = match response.bytes().await {
        Ok(body) => body,
        Err(error) => {
            debug!(
                "Cloudflare resp body failed: method={} url={} status={} error={:?}",
                method, url, status, error
            );
            return Err(error).context("Failed to read Cloudflare response body");
        }
    };
    debug!(
        "Cloudflare resp body: method={} url={} status={} elapsed_ms={} body={:?}",
        method,
        url,
        status,
        start.elapsed().as_millis(),
        String::from_utf8_lossy(&body)
    );
    parse_response(status, &body)
}

fn parse_response<T: DeserializeOwned>(status: StatusCode, body: &[u8]) -> Result<T> {
    // Parse errors independently of result: error envelopes can contain null or missing result.
    let response: CloudflareResponse = serde_json::from_slice(body)
        .with_context(|| format!("Failed to parse Cloudflare response (HTTP {})", status))?;
    if !status.is_success() || !response.success {
        let errors = response
            .errors
            .iter()
            .map(|error| format!("{}: {}", error.code, error.message))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "Cloudflare API error (HTTP {}): {}",
            status,
            if errors.is_empty() {
                "unsuccessful response"
            } else {
                &errors
            }
        );
    }
    let result = response
        .result
        .filter(|value| !value.is_null())
        .ok_or_else(|| anyhow::anyhow!("No result in Cloudflare response (HTTP {})", status))?;
    serde_json::from_value(result).context("Failed to parse Cloudflare result")
}

#[derive(Debug, Serialize)]
struct CreateRecordRequest<'a> {
    #[serde(rename = "type")]
    record_type: &'a str,
    name: &'a str,
    content: &'a str,
    ttl: u32,
    proxied: bool,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse {
    success: bool,
    #[serde(default)]
    errors: Vec<CloudflareError>,
    result: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ProviderConfig {
        toml::from_str("name = 'test'\ntype = 'cloudflare'\nauth = 'key'\nkey = 'secret'\napi_key = 'test-token'\nzone_id = 'zone'").unwrap()
    }

    #[test]
    fn update_only_patches_ip_and_keeps_bearer_auth() {
        let request = update_request(&Client::new(), &config(), "record", "192.0.2.1")
            .build()
            .unwrap();
        assert_eq!(request.method(), reqwest::Method::PATCH);
        assert_eq!(
            request.url().path(),
            "/client/v4/zones/zone/dns_records/record"
        );
        assert_eq!(
            request.headers()[header::AUTHORIZATION],
            "Bearer test-token"
        );
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body, serde_json::json!({ "content": "192.0.2.1" }));
    }

    #[test]
    fn list_escapes_hostname_as_one_exact_filter() {
        let request = list_request(&Client::new(), &config(), "home.example.com&name=other")
            .build()
            .unwrap();
        let query: Vec<_> = request.url().query_pairs().collect();
        assert_eq!(query.len(), 2);
        assert_eq!(query[0], ("type".into(), "A".into()));
        assert_eq!(
            query[1],
            ("name.exact".into(), "home.example.com&name=other".into())
        );
    }

    #[test]
    fn cloudflare_errors_survive_null_or_missing_results() {
        for body in [
            r#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}],"result":null}"#,
            r#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}]}"#,
        ] {
            let error = parse_response::<Vec<DnsRecord>>(StatusCode::FORBIDDEN, body.as_bytes())
                .unwrap_err()
                .to_string();
            assert!(error.contains("403"));
            assert!(error.contains("10000: Authentication error"));
        }
    }

    #[test]
    fn responses_require_http_and_api_success_and_valid_result() {
        let success = br#"{"success":true,"result":{"id":"record","content":"192.0.2.1"}}"#;
        assert_eq!(
            parse_response::<DnsRecord>(StatusCode::OK, success)
                .unwrap()
                .id,
            "record"
        );
        assert!(parse_response::<DnsRecord>(StatusCode::TOO_MANY_REQUESTS, success).is_err());
        assert!(
            parse_response::<DnsRecord>(StatusCode::OK, br#"{"success":false,"result":null}"#)
                .is_err()
        );
        assert!(parse_response::<DnsRecord>(StatusCode::OK, br#"{"success":true}"#).is_err());
        let error =
            parse_response::<DnsRecord>(StatusCode::BAD_GATEWAY, b"<html>Bad gateway</html>")
                .unwrap_err();
        assert!(error.to_string().contains("502"));
        assert!(parse_response::<Vec<DnsRecord>>(
            StatusCode::OK,
            br#"{"success":true,"result":[]}"#
        )
        .unwrap()
        .is_empty());
    }
}
