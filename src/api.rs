use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use log::{info, error, warn};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::provider;

pub struct AppState {
    pub config: Config,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    record_id: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

#[derive(Deserialize)]
struct UpdateQuery {
    key: Option<String>,
}

pub fn create_router(config: Config) -> Router {
    let state = Arc::new(AppState { config });

    Router::new()
        .route("/ddns/{provider}/{host}/{ip}", get(update_dns))
        .route("/health", get(health_check))
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}

async fn access_log(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request info
    let method = request.method().clone();
    let uri = request.uri();
    let path = match uri.query() {
        Some(q) => format!("{}?{}", uri.path(), q),
        None => uri.path().to_string(),
    };
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("-").trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "-".to_string());

    // Process request
    let response = next.run(request).await;

    // Extract response info
    let status = response.status().as_u16();
    let length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let duration = start.elapsed();

    // Access log format: method path "user-agent" ip status length duration
    info!(
        target: "access",
        "{} {} \"{}\" {} {} {} {:.3}ms",
        method, path, user_agent, ip, status, length, duration.as_secs_f64() * 1000.0
    );

    response
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

async fn update_dns(
    State(state): State<Arc<AppState>>,
    Path((provider_name, host, ip)): Path<(String, String, String)>,
    Query(query): Query<UpdateQuery>,
) -> impl IntoResponse {
    // Validate IP address format
    if !is_valid_ipv4(&ip) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: format!("Invalid IP address: {}", ip),
            }),
        )
            .into_response();
    }

    // Find provider config
    let provider_config = match state.config.get_provider(&provider_name) {
        Some(config) => config,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Provider not found: {}", provider_name),
                }),
            )
                .into_response();
        }
    };

    // Verify access key (if configured)
    if let Some(ref config_key) = provider_config.key {
        let request_key = query.key.as_deref().unwrap_or("");
        if request_key != config_key {
            warn!("Invalid key for provider: {}", provider_name);
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    success: false,
                    error: "Invalid key".to_string(),
                }),
            )
                .into_response();
        }
    }

    // Update DNS record based on provider type
    let result = match provider_config.provider_type.as_str() {
        "cloudflare" => provider::cloudflare::update_record(provider_config, &host, &ip).await,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Unsupported provider type: {}", provider_config.provider_type),
                }),
            )
                .into_response();
        }
    };

    match result {
        Ok(result) => {
            info!("DNS update successful: {}", result.message);
            (
                StatusCode::OK,
                Json(ApiResponse {
                    success: result.success,
                    message: result.message,
                    record_id: result.record_id,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("DNS update failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("DNS update failed: {}", e),
                }),
            )
                .into_response()
        }
    }
}

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok()
}
