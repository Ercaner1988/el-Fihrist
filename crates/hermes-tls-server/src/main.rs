//! Hermes TLS Server — Axum HTTP Endpoint (Gemini Task)
//!
//! Bu crate, Gemini'nin talep ettiği "POST /api/call" HTTP endpoint'ini uygular.
//! Hermes'ten gelen ham `CALL:turso_oku(1)` metnini kabul eder, Rust kütüphanesiyle çözer
//! ve sonucu HTTP cevabı olarak geri döndürür.

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use turso_dsl::YetenekYoneticisi;

// --- REQUEST/RESPONSE YAPILARI ---

#[derive(Debug, Deserialize)]
struct CallRequest {
    call: String,
}

#[derive(Debug, Serialize)]
struct CallResponse {
    success: bool,
    result: Option<String>,
    error: Option<String>,
}

// --- STATE (YETENEK YÖNETİCİSİ) ---

struct AppState {
    yetenek_yonetici: YetenekYoneticisi,
}

// --- HTTP ENDPOINT ---

async fn handle_call(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CallRequest>,
) -> Result<Json<CallResponse>, (StatusCode, String)> {
    match state.yetenek_yonetici.cagiriyi_coz_ve_calistir(&req.call) {
        Ok(result) => Ok(Json(CallResponse {
            success: true,
            result: Some(result),
            error: None,
        })),
        Err(e) => {
            let error_msg = format!("{:?}", e);
            Ok(Json(CallResponse {
                success: false,
                result: None,
                error: Some(error_msg),
            }))
        }
    }
}

// --- MAIN ---

#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermes_tls_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = turso_dsl::TursoBaglantisi::yeni();
    let mut yetenek_yonetici = YetenekYoneticisi::yeni();
    yetenek_yonetici.yetenek_ekle(Box::new(turso_dsl::TursoOkuYetenegi::yeni(db.clone())));
    yetenek_yonetici.yetenek_ekle(Box::new(turso_dsl::TursoYazYetenegi::yeni(db.clone())));
    yetenek_yonetici.yetenek_ekle(Box::new(turso_dsl::TursoSilYetenegi::yeni(db)));

    let app_state = Arc::new(AppState { yetenek_yonetici });

    let app = Router::new()
        .route("/api/call", post(handle_call))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("=== Hermes TLS Server Başlatıldı ===");
    println!(
        "Sunucu {} adresinde dinliyor...",
        listener.local_addr().unwrap()
    );
    println!("Endpoint: POST http://127.0.0.1:3000/api/call");
    println!("Örnek Gövde: {{\"call\": \"CALL:turso_oku(1)\"}}");

    axum::serve(listener, app).await.unwrap();
}
