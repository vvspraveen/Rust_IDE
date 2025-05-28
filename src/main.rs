use axum::{
    body::Body,
    extract::Json,
    http::{header, Method, Response, StatusCode},
    response::{IntoResponse, Sse, sse::Event},
    routing::{get, post},
    Router, Server,
};
use futures::Stream;
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tokio_util::io::ReaderStream;
use tower_http::cors::{CorsLayer, Any};

#[derive(Debug, Deserialize)]
struct Prompt {
    prompt: String,
}

// ❗ Shared prompt state (not for production use)
static mut LATEST_PROMPT: Option<String> = None;

// POST /prompt - Store incoming prompt
async fn handle_prompt(Json(payload): Json<Prompt>) -> &'static str {
    unsafe {
        LATEST_PROMPT = Some(payload.prompt.clone());
    }
    "Prompt received"
}

// GET /prompt-stream - Stream LLaMA2 response
async fn handle_stream() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    let prompt = unsafe { LATEST_PROMPT.clone().unwrap_or_else(|| "No prompt sent.".to_string()) };

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let res = client
            .post("http://localhost:11434/api/generate")
            .json(&serde_json::json!({
                "model": "deepseek-coder-v2:16b",
                "prompt": prompt,
                "stream": true
            }))
            .send()
            .await
            .unwrap();

        let mut stream = res.bytes_stream();
        while let Some(Ok(chunk)) = stream.next().await {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&chunk) {
                if let Some(response_text) = json.get("response").and_then(|v| v.as_str()) {
                    let _ = tx.send(Ok(Event::default().data(response_text.to_string()))).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // ✅ Signal frontend to trigger zip download
        let _ = tx.send(Ok(Event::default().data("__DOWNLOAD__".to_string()))).await;
    });

    Sse::new(ReceiverStream::new(rx))
}

// GET /download-zip - Serve dummy.zip file
async fn handle_zip_download() -> Response<Body> {
    let file_path = "dummy.zip";

    match tokio::fs::File::open(file_path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::wrap_stream(stream);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/zip")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"dummy.zip\"")
                .body(body)
                .unwrap()
        }
        Err(_) => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("ZIP file not found"))
                .unwrap()
        }
    }
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/prompt", post(handle_prompt))
        .route("/prompt-stream", get(handle_stream))
        .route("/download-zip", get(handle_zip_download))
        .layer(cors);

    println!("🚀 Backend running at http://localhost:3000");
    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

