use anyhow::Result;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info, warn};

type HmacSha256 = Hmac<Sha256>;

struct WebhookState {
    run_id: u64,
    secret: String,
    tx: Mutex<Option<oneshot::Sender<Value>>>,
}

/// Start a local webhook server for GitHub events
///
/// Listens for workflow_run events from GitHub.
/// GitHub must be configured to send webhooks to this address.
pub async fn start_webhook_server(
    port: u16,
    run_id: u64,
    secret: String,
) -> Result<(oneshot::Receiver<Value>, oneshot::Sender<()>)> {
    let (event_tx, event_rx) = oneshot::channel::<Value>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let state = Arc::new(WebhookState {
        run_id,
        secret,
        tx: Mutex::new(Some(event_tx)),
    });

    let app = Router::new()
        .route("/webhook/github", post(handle_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Webhook listener started on http://{}/webhook/github", addr);

    tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            warn!("Webhook server terminated with error: {e}");
        }
    });

    Ok((event_rx, shutdown_tx))
}

/// Await a specific workflow run to complete via webhook
pub async fn await_run_via_webhook(
    _repo: &str,
    run_id: u64,
    timeout_secs: u64,
    port: u16,
    secret: &str,
) -> Result<crate::awaiting::WorkflowRun> {
    let (event_rx, shutdown_tx) = start_webhook_server(port, run_id, secret.to_string()).await?;

    let event = match tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs.max(1)),
        event_rx,
    )
    .await
    {
        Ok(Ok(run_json)) => run_json,
        Ok(Err(_)) => anyhow::bail!("webhook listener closed before receiving matching event"),
        Err(_) => anyhow::bail!("timed out waiting for webhook event after {} seconds", timeout_secs),
    };

    let _ = shutdown_tx.send(());
    crate::awaiting::WorkflowRun::from_api_response(&event)
}

async fn handle_webhook(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    let Some(signature) = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
    else {
        return (StatusCode::UNAUTHORIZED, "missing signature");
    };

    let Some(event_name) = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
    else {
        return (StatusCode::BAD_REQUEST, "missing event type");
    };

    if event_name != "workflow_run" {
        return (StatusCode::ACCEPTED, "ignored");
    }

    if !verify_signature(signature, &body, &state.secret) {
        return (StatusCode::UNAUTHORIZED, "bad signature");
    }

    let Ok(payload): Result<Value, _> = serde_json::from_slice(&body) else {
        return (StatusCode::BAD_REQUEST, "invalid json");
    };

    let action = payload["action"].as_str().unwrap_or_default();
    if action != "completed" {
        return (StatusCode::ACCEPTED, "ignored non-terminal event");
    }

    let run = payload["workflow_run"].clone();
    let Some(run_id) = run["id"].as_u64() else {
        return (StatusCode::BAD_REQUEST, "missing workflow_run.id");
    };

    if run_id != state.run_id {
        debug!("Ignoring webhook for run {} while awaiting {}", run_id, state.run_id);
        return (StatusCode::ACCEPTED, "run id mismatch");
    }

    let mut tx_guard = state.tx.lock().await;
    if let Some(tx) = tx_guard.take() {
        let _ = tx.send(run);
    }
    (StatusCode::OK, "accepted")
}

fn verify_signature(signature: &str, body: &[u8], secret: &str) -> bool {
    let provided = signature.strip_prefix("sha256=").unwrap_or_default();

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    let expected_hex = hex::encode(mac.finalize().into_bytes());

    subtle::ConstantTimeEq::ct_eq(provided.as_bytes(), expected_hex.as_bytes()).into()
}

