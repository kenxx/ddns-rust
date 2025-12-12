use anyhow::{Context, Result};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;
use super::DnsUpdateResult;

const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";

pub async fn update_record(config: &ProviderConfig, host: &str, ip: &str) -> Result<DnsUpdateResult> {
    let client = Client::new();

    // Check if record exists
    if let Some(existing) = get_record(&client, config, host).await? {
        if existing.content == ip {
            info!("Record {} already has IP {}, no update needed", host, ip);
            return Ok(DnsUpdateResult {
                success: true,
                message: format!("Record already up to date with IP {}", ip),
                record_id: Some(existing.id),
            });
        }

        info!("Updating existing record {} from {} to {}", host, existing.content, ip);
        let record = update_existing_record(&client, config, &existing.id, host, ip).await?;

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

async fn get_record(client: &Client, config: &ProviderConfig, host: &str) -> Result<Option<DnsRecord>> {
    let url = format!(
        "{}/zones/{}/dns_records?type=A&name={}",
        CLOUDFLARE_API_BASE, config.zone_id, host
    );

    let response: CloudflareListResponse = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to send request to Cloudflare")?
        .json()
        .await
        .context("Failed to parse Cloudflare response")?;

    if !response.success {
        let errors: Vec<String> = response
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect();
        anyhow::bail!("Cloudflare API error: {}", errors.join(", "));
    }

    Ok(response.result.into_iter().next())
}

async fn create_record(client: &Client, config: &ProviderConfig, host: &str, ip: &str) -> Result<DnsRecord> {
    let url = format!(
        "{}/zones/{}/dns_records",
        CLOUDFLARE_API_BASE, config.zone_id
    );

    let body = CreateRecordRequest {
        record_type: "A".to_string(),
        name: host.to_string(),
        content: ip.to_string(),
        ttl: 1,
        proxied: false,
    };

    let response: CloudflareResponse = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to send create request to Cloudflare")?
        .json()
        .await
        .context("Failed to parse Cloudflare create response")?;

    if !response.success {
        let errors: Vec<String> = response
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect();
        anyhow::bail!("Cloudflare API error: {}", errors.join(", "));
    }

    response
        .result
        .ok_or_else(|| anyhow::anyhow!("No result in Cloudflare response"))
}

async fn update_existing_record(
    client: &Client,
    config: &ProviderConfig,
    record_id: &str,
    host: &str,
    ip: &str,
) -> Result<DnsRecord> {
    let url = format!(
        "{}/zones/{}/dns_records/{}",
        CLOUDFLARE_API_BASE, config.zone_id, record_id
    );

    let body = UpdateRecordRequest {
        record_type: "A".to_string(),
        name: host.to_string(),
        content: ip.to_string(),
        ttl: 1,
        proxied: false,
    };

    let response: CloudflareResponse = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to send update request to Cloudflare")?
        .json()
        .await
        .context("Failed to parse Cloudflare update response")?;

    if !response.success {
        let errors: Vec<String> = response
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect();
        anyhow::bail!("Cloudflare API error: {}", errors.join(", "));
    }

    response
        .result
        .ok_or_else(|| anyhow::anyhow!("No result in Cloudflare response"))
}

// Cloudflare API types

#[derive(Debug, Serialize)]
struct CreateRecordRequest {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Debug, Serialize)]
struct UpdateRecordRequest {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse {
    success: bool,
    #[serde(default)]
    errors: Vec<CloudflareError>,
    result: Option<DnsRecord>,
}

#[derive(Debug, Deserialize)]
struct CloudflareListResponse {
    success: bool,
    #[serde(default)]
    errors: Vec<CloudflareError>,
    result: Vec<DnsRecord>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    record_type: String,
    #[allow(dead_code)]
    name: String,
    content: String,
}
