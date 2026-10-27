use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::{to_bytes, Body},
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::config::{AuthConfig, Config};
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

async fn access_log(mut request: Request, next: Next) -> Response {
    let start = Instant::now();

    if log::log_enabled!(log::Level::Debug) {
        // Intentionally includes credentials for diagnosing router authentication.
        debug!("Incoming request: {:?}", request);

        let (parts, body) = request.into_parts();
        match to_bytes(body, usize::MAX).await {
            Ok(body) => {
                debug!(
                    "Request body ({} bytes): text={:?}, bytes={:?}",
                    body.len(),
                    String::from_utf8_lossy(&body),
                    body
                );
                // Restore the body so logging does not consume handler input.
                request = Request::from_parts(parts, Body::from(body));
            }
            Err(error) => {
                debug!("Failed to read request body: {:?}", error);
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        }
    }

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
    headers: HeaderMap,
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

    if !is_authorized(&provider_config.auth, &headers, query.key.as_deref()) {
        warn!("Authentication failed for provider: {}", provider_name);
        let mut response = (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }),
        )
            .into_response();
        if matches!(provider_config.auth, AuthConfig::Basic { .. }) {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"ddns\"".parse().unwrap(),
            );
        }
        return response;
    }

    // Update DNS record based on provider type
    let result = match provider_config.provider_type.as_str() {
        "cloudflare" => provider::cloudflare::update_record(provider_config, &host, &ip).await,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: format!(
                        "Unsupported provider type: {}",
                        provider_config.provider_type
                    ),
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

fn is_authorized(auth: &AuthConfig, headers: &HeaderMap, query_key: Option<&str>) -> bool {
    match auth {
        AuthConfig::Key { key } => !key.is_empty() && query_key == Some(key.as_str()),
        AuthConfig::Basic { username, password } => {
            if username.is_empty() || password.is_empty() {
                return false;
            }
            let mut values = headers.get_all(header::AUTHORIZATION).iter();
            let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
                return false;
            };
            if values.next().is_some() {
                return false;
            }
            let Some((scheme, encoded)) = value.split_once(' ') else {
                return false;
            };
            if !scheme.eq_ignore_ascii_case("basic") {
                return false;
            }
            let Ok(decoded) = STANDARD.decode(encoded.trim_start_matches(' ')) else {
                return false;
            };
            let Ok(credentials) = std::str::from_utf8(&decoded) else {
                return false;
            };
            credentials.split_once(':') == Some((username.as_str(), password.as_str()))
        }
    }
}

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_headers(scheme: &str, credentials: &[u8]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("{scheme} {}", STANDARD.encode(credentials))
                .parse()
                .unwrap(),
        );
        headers
    }

    #[test]
    fn basic_checks_credentials_and_never_accepts_query_key() {
        let auth = AuthConfig::Basic {
            username: "user".into(),
            password: "pass:word".into(),
        };
        assert!(is_authorized(
            &auth,
            &basic_headers("Basic", b"user:pass:word"),
            None
        ));
        assert!(is_authorized(
            &auth,
            &basic_headers("bAsIc", b"user:pass:word"),
            Some("wrong")
        ));
        for credentials in [
            b"user:wrong".as_slice(),
            b"other:pass:word",
            b"user",
            b"user:\xff",
        ] {
            assert!(!is_authorized(
                &auth,
                &basic_headers("Basic", credentials),
                None
            ));
        }
        assert!(!is_authorized(&auth, &HeaderMap::new(), Some("secret")));
        assert!(!is_authorized(
            &auth,
            &basic_headers("Bearer", b"user:pass:word"),
            None
        ));
        let mut malformed = HeaderMap::new();
        malformed.insert(header::AUTHORIZATION, "Basic !!!".parse().unwrap());
        assert!(!is_authorized(&auth, &malformed, None));
        let mut duplicate = basic_headers("Basic", b"user:pass:word");
        duplicate.append(header::AUTHORIZATION, "Basic !!!".parse().unwrap());
        assert!(!is_authorized(&auth, &duplicate, None));
    }

    #[test]
    fn key_requires_query_key_even_with_valid_basic_header() {
        let auth = AuthConfig::Key {
            key: "secret".into(),
        };
        let headers = basic_headers("Basic", b"user:secret");
        assert!(is_authorized(&auth, &HeaderMap::new(), Some("secret")));
        assert!(!is_authorized(&auth, &headers, None));
        assert!(!is_authorized(&auth, &headers, Some("wrong")));
    }

    #[tokio::test]
    async fn basic_rejection_has_challenge_and_valid_credentials_reach_handler() {
        // An unsupported DNS provider avoids real DNS changes and marks auth success.
        let config: Config = toml::from_str(
            "[[providers]]\nname = 'test'\ntype = 'test-only'\napi_key = 'token'\nzone_id = 'zone'\nauth = 'basic'\nusername = 'user'\npassword = 'secret'"
        ).unwrap();
        let state = Arc::new(AppState { config });
        for (headers, status) in [
            (HeaderMap::new(), StatusCode::UNAUTHORIZED),
            (
                basic_headers("Basic", b"user:wrong"),
                StatusCode::UNAUTHORIZED,
            ),
            (
                basic_headers("Basic", b"user:secret"),
                StatusCode::BAD_REQUEST,
            ),
        ] {
            let response = update_dns(
                State(state.clone()),
                Path(("test".into(), "home.example.com".into(), "192.0.2.1".into())),
                Query(UpdateQuery { key: None }),
                headers,
            )
            .await
            .into_response();
            assert_eq!(response.status(), status);
            if status == StatusCode::UNAUTHORIZED {
                assert_eq!(
                    response.headers()[header::WWW_AUTHENTICATE],
                    "Basic realm=\"ddns\""
                );
            }
        }
    }
}
