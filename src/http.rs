use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream;
use serde_json::Value;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

use crate::bind::{host_from_env, listen_addr, port_from_env};
use crate::logging::Logger;
use crate::parse::{parse_start_request, StartRequestError};
use crate::ports::{list_ports, SerialOpener};
use crate::session::{Session, SessionError};
use crate::{health_body, SERVICE_NAME};

#[derive(Clone)]
struct AppState {
  session: Arc<Session>,
}

pub async fn serve(session: Arc<Session>) -> Result<(), String> {
  let host = host_from_env();
  let port = port_from_env()?;
  let addr = listen_addr(&host, port);
  let listener = tokio::net::TcpListener::bind(addr)
    .await
    .map_err(|error| format!("Failed to bind {addr}: {error}"))?;
  let bound = listener.local_addr().unwrap_or(addr);
  eprintln!("INFO ham-radio-sniffer {SERVICE_NAME} listening on http://{bound}");

  let shutdown_session = Arc::clone(&session);
  axum::serve(listener, router(session))
    .with_graceful_shutdown(shutdown_signal(shutdown_session))
    .await
    .map_err(|error| error.to_string())
}

pub fn router(session: Arc<Session>) -> Router {
  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    .allow_headers([header::CONTENT_TYPE]);

  Router::new()
    .route("/", get(health))
    .route("/api/health", get(health))
    .route("/api/ports", get(ports))
    .route("/api/sniffer", get(sniffer_status))
    .route("/api/sniffer/start", post(start))
    .route("/api/sniffer/stop", post(stop))
    .route("/api/sniffer/log", get(log))
    .route("/api/sniffer/events", get(events))
    .layer(cors)
    .with_state(AppState { session })
}

pub fn session_from_env() -> Arc<Session> {
  Session::new(Arc::new(SerialOpener), Logger::from_env())
}

async fn health() -> Json<Value> {
  Json(health_body())
}

async fn ports() -> Response {
  match list_ports() {
    Ok(ports) => Json(serde_json::json!({ "ports": ports })).into_response(),
    Err(error) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &error),
  }
}

async fn sniffer_status(State(state): State<AppState>) -> Json<Value> {
  Json(state.session.status())
}

async fn start(
  State(state): State<AppState>,
  body: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
  let Json(body) = match body {
    Ok(body) => body,
    Err(_) => {
      return error_response(
        StatusCode::BAD_REQUEST,
        "Request body must be a JSON object",
      )
    }
  };

  let request = match parse_start_request(&body) {
    Ok(request) => request,
    Err(StartRequestError { message }) => return error_response(StatusCode::BAD_REQUEST, &message),
  };

  match state.session.start(request) {
    Ok(status) => Json(status).into_response(),
    Err(error) => session_error(error),
  }
}

async fn stop(State(state): State<AppState>) -> Json<Value> {
  Json(state.session.stop())
}

async fn log(State(state): State<AppState>) -> Json<Value> {
  Json(state.session.log_response())
}

async fn events(
  State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>> + Send> {
  let rx = state.session.subscribe();
  let initial = serde_json::json!({
    "type": "status",
    "status": state.session.status(),
  })
  .to_string();

  let stream = stream::unfold((Some(initial), rx), |(pending, mut rx)| async move {
    if let Some(payload) = pending {
      return Some((Ok(Event::default().data(payload)), (None, rx)));
    }

    loop {
      match rx.recv().await {
        Ok(payload) => return Some((Ok(Event::default().data(payload)), (None, rx))),
        Err(broadcast::error::RecvError::Lagged(_)) => continue,
        Err(broadcast::error::RecvError::Closed) => return None,
      }
    }
  });

  Sse::new(stream)
    .keep_alive(axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

fn session_error(error: SessionError) -> Response {
  let status = match error.status_code() {
    409 => StatusCode::CONFLICT,
    _ => StatusCode::BAD_REQUEST,
  };
  error_response(status, error.message())
}

fn error_response(status: StatusCode, message: &str) -> Response {
  let mut response = (
    status,
    Json(serde_json::json!({
      "statusCode": status.as_u16(),
      "statusMessage": message,
      "message": message,
    })),
  )
    .into_response();
  response.headers_mut().insert(
    header::ACCESS_CONTROL_ALLOW_ORIGIN,
    HeaderValue::from_static("*"),
  );
  response
}

async fn shutdown_signal(session: Arc<Session>) {
  let ctrl_c = async {
    let _ = tokio::signal::ctrl_c().await;
  };

  #[cfg(unix)]
  let terminate = async {
    if let Ok(mut signal) =
      tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
      signal.recv().await;
    }
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    _ = ctrl_c => {},
    _ = terminate => {},
  }

  session.stop();
}

pub async fn serve_from_env() -> Result<(), String> {
  serve(session_from_env()).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bridge::{PortEnds, PortOpener, PortReader, PortWriter};
  use axum::body::Body;
  use http_body_util::BodyExt;
  use tower::ServiceExt;

  struct IdleReader;

  impl PortReader for IdleReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
      std::thread::sleep(std::time::Duration::from_millis(20));
      Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "idle"))
    }
  }

  struct Sink;

  impl PortWriter for Sink {
    fn write_all(&mut self, _buf: &[u8]) -> std::io::Result<()> {
      Ok(())
    }
  }

  struct OkOpener;

  impl PortOpener for OkOpener {
    fn open(
      &self,
      _path: &str,
      _baud_rate: u32,
      _rts: bool,
      _dtr: bool,
    ) -> Result<PortEnds, String> {
      Ok(PortEnds {
        reader: Box::new(IdleReader),
        writer: Box::new(Sink),
      })
    }
  }

  fn app() -> Router {
    router(Session::new(Arc::new(OkOpener), Logger::quiet()))
  }

  async fn json_body(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
  }

  #[tokio::test]
  async fn health_reports_the_crate_version() {
    let response = app()
      .oneshot(
        axum::http::Request::builder()
          .uri("/api/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      json_body(response).await,
      serde_json::json!({
        "ok": true,
        "service": "ham-radio-sniffer",
        "version": env!("CARGO_PKG_VERSION"),
      })
    );
  }

  #[tokio::test]
  async fn root_matches_health() {
    let response = app()
      .oneshot(
        axum::http::Request::builder()
          .uri("/")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    let body = json_body(response).await;
    assert_eq!(body["service"], "ham-radio-sniffer");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
  }

  #[tokio::test]
  async fn start_rejects_an_invalid_body_and_a_second_session() {
    let session = Session::new(Arc::new(OkOpener), Logger::quiet());
    let app = router(Arc::clone(&session));

    let missing = app
      .clone()
      .oneshot(
        axum::http::Request::builder()
          .method("POST")
          .uri("/api/sniffer/start")
          .header("content-type", "application/json")
          .body(Body::from(r#"{"radioPort":"/dev/ttyUSB0"}"#))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
      json_body(missing).await["statusMessage"],
      "computerPort is required"
    );

    let started = app
      .clone()
      .oneshot(
        axum::http::Request::builder()
          .method("POST")
          .uri("/api/sniffer/start")
          .header("content-type", "application/json")
          .body(Body::from(
            r#"{"computerPort":"/dev/computer","radioPort":"/dev/radio","baudRate":9600}"#,
          ))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(started.status(), StatusCode::OK);
    let status = json_body(started).await;
    assert_eq!(status["running"], true);
    assert_eq!(status["baudRate"], 9600);

    let conflict = app
      .clone()
      .oneshot(
        axum::http::Request::builder()
          .method("POST")
          .uri("/api/sniffer/start")
          .header("content-type", "application/json")
          .body(Body::from(
            r#"{"computerPort":"/dev/computer","radioPort":"/dev/radio"}"#,
          ))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
      json_body(conflict).await["statusMessage"],
      "Sniffer is already running"
    );

    session.stop();
    let _ = std::fs::remove_file(status["logFile"].as_str().unwrap_or("radio-sniffer.json"));
  }
}
