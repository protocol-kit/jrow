//! Middleware integration tests

use jrow_server::{JrowServer, from_fn, LoggingMiddleware, MetricsMiddleware};

#[tokio::test]
async fn test_middleware_logging() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let server = JrowServer::builder()
        .bind(addr)
        .use_middleware(LoggingMiddleware)
        .handler("test", from_fn(|_| async {
            Ok(serde_json::json!({"result": "ok"}))
        }))
        .build()
        .await
        .unwrap();
    
    assert!(server.local_addr().is_ok());
}

#[tokio::test]
async fn test_middleware_metrics() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let server = JrowServer::builder()
        .bind(addr)
        .use_middleware(MetricsMiddleware)
        .handler("test", from_fn(|_| async {
            Ok(serde_json::json!({"result": "ok"}))
        }))
        .build()
        .await
        .unwrap();
    
    assert!(server.local_addr().is_ok());
}

#[tokio::test]
async fn test_middleware_chain() {
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    
    let server = JrowServer::builder()
        .bind(addr)
        .use_middleware(LoggingMiddleware)
        .use_middleware(MetricsMiddleware)
        .handler("test", from_fn(|_| async {
            Ok(serde_json::json!({"result": "ok"}))
        }))
        .build()
        .await
        .unwrap();
    
    assert!(server.local_addr().is_ok());
}

