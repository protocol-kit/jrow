//! JROW Server with Embedded Web UI - Feature Showcase
//!
//! This example demonstrates all major JROW features:
//! 1. Serves the web UI over HTTP (http://localhost:8080)
//! 2. Handles WebSocket JSON-RPC connections (ws://localhost:8081)
//! 3. RPC methods (math, string operations, user management)
//! 4. Pub/Sub with periodic server-initiated messages
//! 5. Error handling examples
//! 6. Notifications
//!
//! Run with:
//!   cargo run --example server_with_ui
//!
//! Then open http://localhost:8080 in your browser!

use async_trait::async_trait;
use jrow_core::Result as JrowResult;
use jrow_server::{from_fn, from_typed_fn, JrowServer, Middleware, MiddlewareAction, MiddlewareContext, RetentionPolicy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use warp::Filter;

// ============================================================================
// Response Logging Middleware
// ============================================================================

/// Middleware to log responses and send them via channel
struct ResponseLoggingMiddleware {
    log_tx: mpsc::UnboundedSender<serde_json::Value>,
}

impl ResponseLoggingMiddleware {
    fn new(log_tx: mpsc::UnboundedSender<serde_json::Value>) -> Self {
        Self { log_tx }
    }
}

#[async_trait]
impl Middleware for ResponseLoggingMiddleware {
    async fn pre_handle(&self, _ctx: &mut MiddlewareContext) -> JrowResult<MiddlewareAction> {
        // Don't do anything before the handler
        Ok(MiddlewareAction::Continue)
    }

    async fn post_handle(&self, ctx: &mut MiddlewareContext, result: &JrowResult<serde_json::Value>) -> JrowResult<()> {
        // Create log entry
        let log_data = match result {
            Ok(value) => {
                // Truncate large responses for logging
                let value_str = value.to_string();
                let display_value = if value_str.len() > 100 {
                    format!("{}...", &value_str[..100])
                } else {
                    value_str
                };

                serde_json::json!({
                    "level": "success",
                    "message": format!("✓ Response: {} → {}", ctx.method, display_value),
                    "method": ctx.method,
                    "conn_id": ctx.conn_id,
                    "request_id": ctx.request_id,
                    "status": "success",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            }
            Err(err) => {
                serde_json::json!({
                    "level": "error",
                    "message": format!("✗ Error: {} → {}", ctx.method, err),
                    "method": ctx.method,
                    "conn_id": ctx.conn_id,
                    "request_id": ctx.request_id,
                    "status": "error",
                    "error": err.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            }
        };

        // Send to channel (ignore if channel is closed)
        let _ = self.log_tx.send(log_data);
        
        Ok(())
    }
}

// ============================================================================
// RPC Types - Math Operations
// ============================================================================

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct AddResult {
    sum: i32,
}

#[derive(Deserialize)]
struct SubtractParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct SubtractResult {
    difference: i32,
}

#[derive(Deserialize)]
struct MultiplyParams {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct MultiplyResult {
    product: i32,
}

#[derive(Deserialize)]
struct DivideParams {
    a: f64,
    b: f64,
}

#[derive(Serialize)]
struct DivideResult {
    quotient: f64,
}

// ============================================================================
// RPC Types - String Operations
// ============================================================================

#[derive(Deserialize)]
struct EchoParams {
    message: String,
}

#[derive(Serialize)]
struct EchoResult {
    echo: String,
    length: usize,
    timestamp: String,
}

#[derive(Deserialize)]
struct ReverseParams {
    text: String,
}

#[derive(Serialize)]
struct ReverseResult {
    original: String,
    reversed: String,
}

#[derive(Deserialize)]
struct ToUpperParams {
    text: String,
}

#[derive(Serialize)]
struct ToUpperResult {
    result: String,
}

// ============================================================================
// RPC Types - User Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
    role: String,
}

#[derive(Deserialize)]
struct CreateUserParams {
    name: String,
    email: String,
    role: String,
}

#[derive(Serialize)]
struct CreateUserResult {
    user: User,
    message: String,
}

#[derive(Deserialize)]
struct GetUserParams {
    id: u32,
}

#[derive(Serialize)]
struct GetUserResult {
    user: Option<User>,
}

#[derive(Serialize)]
struct ListUsersResult {
    users: Vec<User>,
    count: usize,
}

// ============================================================================
// Shared State
// ============================================================================

type UserStore = Arc<RwLock<HashMap<u32, User>>>;

struct AppState {
    users: UserStore,
    next_user_id: Arc<RwLock<u32>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting JROW Server with Web UI...\n");

    // Get the web-ui directory path
    let web_ui_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web-ui");
    
    if !web_ui_dir.exists() {
        eprintln!("❌ Error: web-ui directory not found at: {:?}", web_ui_dir);
        eprintln!("Make sure you're running from the project root directory.");
        std::process::exit(1);
    }

    // Create HTTP routes for serving static files
    let index_route = warp::path::end()
        .and(warp::fs::file(web_ui_dir.join("index.html")));
    
    let static_files = warp::fs::dir(web_ui_dir.clone());

    // Combine routes
    let http_routes = index_route.or(static_files);

    let http_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    
    println!("📡 Starting HTTP server for Web UI...");
    println!("   URL: http://{}", http_addr);
    println!();

    // Spawn HTTP server
    tokio::spawn(async move {
        warp::serve(http_routes)
            .run(http_addr)
            .await;
    });

    // Create shared state
    let app_state = AppState {
        users: Arc::new(RwLock::new(HashMap::new())),
        next_user_id: Arc::new(RwLock::new(1)),
    };

    // Seed some initial users
    {
        let mut users = app_state.users.write().await;
        users.insert(1, User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "admin".to_string(),
        });
        users.insert(2, User {
            id: 2,
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "user".to_string(),
        });
        *app_state.next_user_id.write().await = 3;
    }

    // Create WebSocket address for JROW
    let ws_addr = "127.0.0.1:8081";
    
    println!("🔌 Starting JROW WebSocket server...");
    println!("   WebSocket URL: ws://{}", ws_addr);
    println!();

    // ========================================================================
    // Math Operation Handlers
    // ========================================================================

    let add_handler = from_typed_fn(|params: AddParams| async move {
        Ok(AddResult {
            sum: params.a + params.b,
        })
    });

    let subtract_handler = from_typed_fn(|params: SubtractParams| async move {
        Ok(SubtractResult {
            difference: params.a - params.b,
        })
    });

    let multiply_handler = from_typed_fn(|params: MultiplyParams| async move {
        Ok(MultiplyResult {
            product: params.a * params.b,
        })
    });

    let divide_handler = from_typed_fn(|params: DivideParams| async move {
        if params.b == 0.0 {
            return Err(jrow_core::Error::InvalidParams(
                "Division by zero is not allowed".to_string()
            ));
        }
        Ok(DivideResult {
            quotient: params.a / params.b,
        })
    });

    // ========================================================================
    // String Operation Handlers
    // ========================================================================

    let echo_handler = from_typed_fn(|params: EchoParams| async move {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Ok(EchoResult {
            length: params.message.len(),
            echo: params.message,
            timestamp,
        })
    });

    let reverse_handler = from_typed_fn(|params: ReverseParams| async move {
        let reversed: String = params.text.chars().rev().collect();
        Ok(ReverseResult {
            original: params.text,
            reversed,
        })
    });

    let to_upper_handler = from_typed_fn(|params: ToUpperParams| async move {
        Ok(ToUpperResult {
            result: params.text.to_uppercase(),
        })
    });

    // ========================================================================
    // User Management Handlers
    // ========================================================================

    let users_for_create = app_state.users.clone();
    let next_id_for_create = app_state.next_user_id.clone();
    let create_user_handler = from_fn(move |params: Option<serde_json::Value>| {
        let users = users_for_create.clone();
        let next_id = next_id_for_create.clone();
        
        async move {
            let params: CreateUserParams = serde_json::from_value(
                params.ok_or_else(|| jrow_core::Error::InvalidParams("Missing parameters".to_string()))?
            ).map_err(|e| jrow_core::Error::InvalidParams(e.to_string()))?;

            let mut users_lock = users.write().await;
            let mut next_id_lock = next_id.write().await;
            
            let id = *next_id_lock;
            *next_id_lock += 1;

            let user = User {
                id,
                name: params.name,
                email: params.email,
                role: params.role,
            };

            users_lock.insert(id, user.clone());

            Ok(serde_json::to_value(CreateUserResult {
                user,
                message: format!("User created successfully with ID {}", id),
            }).unwrap())
        }
    });

    let users_for_get = app_state.users.clone();
    let get_user_handler = from_fn(move |params: Option<serde_json::Value>| {
        let users = users_for_get.clone();
        
        async move {
            let params: GetUserParams = serde_json::from_value(
                params.ok_or_else(|| jrow_core::Error::InvalidParams("Missing parameters".to_string()))?
            ).map_err(|e| jrow_core::Error::InvalidParams(e.to_string()))?;

            let users_lock = users.read().await;
            let user = users_lock.get(&params.id).cloned();

            Ok(serde_json::to_value(GetUserResult { user }).unwrap())
        }
    });

    let users_for_list = app_state.users.clone();
    let list_users_handler = from_fn(move |_params: Option<serde_json::Value>| {
        let users = users_for_list.clone();
        
        async move {
            let users_lock = users.read().await;
            let user_list: Vec<User> = users_lock.values().cloned().collect();
            let count = user_list.len();

            Ok(serde_json::to_value(ListUsersResult {
                users: user_list,
                count,
            }).unwrap())
        }
    });

    // ========================================================================
    // Special Handlers
    // ========================================================================

    // Handler that demonstrates async delay
    let slow_operation_handler = from_fn(|_params: Option<serde_json::Value>| async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(serde_json::json!({
            "status": "completed",
            "message": "This operation took 2 seconds",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    });

    // Handler that always returns an error (for testing error handling)
    let error_test_handler = from_fn(|_params: Option<serde_json::Value>| async move {
        Err(jrow_core::Error::Internal(
            "This is a test error for demonstrating error handling".to_string()
        ))
    });

    // ========================================================================
    // Notification Handlers (Client -> Server, no response)
    // ========================================================================

    // Create channel for logging (both responses and notifications)
    let (log_tx, mut log_rx) = mpsc::unbounded_channel::<serde_json::Value>();

    // Handler for log notifications
    let log_tx_clone = log_tx.clone();
    let log_handler = from_fn(move |params: Option<serde_json::Value>| {
        let log_tx = log_tx_clone.clone();
        async move {
            if let Some(params) = params {
                let message = if let Some(message) = params.get("message") {
                    println!("📝 Client log: {}", message);
                    message.to_string()
                } else {
                    println!("📝 Client notification: {}", params);
                    params.to_string()
                };
                
                // Publish to server.logs
                let log_data = serde_json::json!({
                    "level": "info",
                    "message": format!("📝 Client notification: log → {}", message),
                    "method": "log",
                    "type": "notification",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                let _ = log_tx.send(log_data);
            }
            Ok(serde_json::Value::Null)
        }
    });

    // Handler for telemetry/analytics notifications
    let log_tx_clone = log_tx.clone();
    let telemetry_handler = from_fn(move |params: Option<serde_json::Value>| {
        let log_tx = log_tx_clone.clone();
        async move {
            if let Some(params) = params {
                println!("📊 Telemetry data received: {}", params);
                
                // Truncate large telemetry data
                let params_str = params.to_string();
                let display_params = if params_str.len() > 100 {
                    format!("{}...", &params_str[..100])
                } else {
                    params_str
                };
                
                // Publish to server.logs
                let log_data = serde_json::json!({
                    "level": "info",
                    "message": format!("📊 Client notification: telemetry → {}", display_params),
                    "method": "telemetry",
                    "type": "notification",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                let _ = log_tx.send(log_data);
            }
            Ok(serde_json::Value::Null)
        }
    });

    // Handler for heartbeat/ping notifications
    let log_tx_clone = log_tx.clone();
    let heartbeat_handler = from_fn(move |params: Option<serde_json::Value>| {
        let log_tx = log_tx_clone.clone();
        async move {
            let timestamp = if let Some(p) = &params {
                p.get("timestamp")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                "unknown".to_string()
            };
            println!("💓 Heartbeat received at {}", timestamp);
            
            // Publish to server.logs
            let log_data = serde_json::json!({
                "level": "debug",
                "message": format!("💓 Client notification: heartbeat → {}", timestamp),
                "method": "heartbeat",
                "type": "notification",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            let _ = log_tx.send(log_data);
            
            Ok(serde_json::Value::Null)
        }
    });

    // Handler for user activity tracking
    let log_tx_clone = log_tx.clone();
    let activity_handler = from_fn(move |params: Option<serde_json::Value>| {
        let log_tx = log_tx_clone.clone();
        async move {
            if let Some(params) = &params {
                let action = params.get("action")
                    .and_then(|a| a.as_str())
                    .unwrap_or("unknown");
                let user = params.get("user")
                    .and_then(|u| u.as_str())
                    .unwrap_or("anonymous");
                println!("👤 User activity: {} performed '{}'", user, action);
                
                // Publish to server.logs
                let log_data = serde_json::json!({
                    "level": "info",
                    "message": format!("👤 Client notification: activity → {} (user: {})", action, user),
                    "method": "activity",
                    "type": "notification",
                    "user": user,
                    "action": action,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                let _ = log_tx.send(log_data);
            }
            Ok(serde_json::Value::Null)
        }
    });
    
    // Create response logging middleware (uses the same channel as notification handlers)
    let response_logger = ResponseLoggingMiddleware::new(log_tx.clone());

    // Build JROW server
    let server = JrowServer::builder()
        .bind_str(ws_addr)?
        // Enable persistent subscriptions with durable storage
        .with_persistent_storage("./data/server_with_ui.db")
        // Register topics with retention policies
        .register_topic(
            "persistent.events",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(3600)), // 1 hour
                max_count: Some(1000),
                max_bytes: Some(10 * 1024 * 1024), // 10MB
            },
        )
        .register_topic(
            "persistent.notifications",
            RetentionPolicy {
                max_age: Some(Duration::from_secs(7200)), // 2 hours
                max_count: Some(500),
                max_bytes: Some(5 * 1024 * 1024), // 5MB
            },
        )
        .subscription_timeout(Duration::from_secs(300)) // 5 minutes
        .retention_interval(Duration::from_secs(60)) // Clean up every minute
        // Add response logging middleware
        .use_middleware(Arc::new(response_logger))
        // Math operations
        .handler("add", add_handler)
        .handler("subtract", subtract_handler)
        .handler("multiply", multiply_handler)
        .handler("divide", divide_handler)
        // String operations
        .handler("echo", echo_handler)
        .handler("reverse", reverse_handler)
        .handler("toUpper", to_upper_handler)
        // User management
        .handler("user.create", create_user_handler)
        .handler("user.get", get_user_handler)
        .handler("user.list", list_users_handler)
        // Special operations
        .handler("slowOperation", slow_operation_handler)
        .handler("testError", error_test_handler)
        // Client notification handlers (no response sent back)
        .handler("log", log_handler)
        .handler("telemetry", telemetry_handler)
        .handler("heartbeat", heartbeat_handler)
        .handler("activity", activity_handler)
        .build()
        .await?;

    println!("✅ Server ready!");
    println!();
    println!("📖 Open in browser: http://127.0.0.1:8080");
    println!("🔗 WebSocket URL:   ws://127.0.0.1:8081");
    println!();
    println!("💡 In the web UI, connect to: ws://127.0.0.1:8081");
    println!();
    println!("📝 Available RPC Methods:");
    println!();
    println!("   🧮 Math Operations:");
    println!("      • add(a, b)          - Add two numbers");
    println!("      • subtract(a, b)     - Subtract two numbers");
    println!("      • multiply(a, b)     - Multiply two numbers");
    println!("      • divide(a, b)       - Divide two numbers (errors if b=0)");
    println!();
    println!("   📝 String Operations:");
    println!("      • echo(message)      - Echo message with metadata");
    println!("      • reverse(text)      - Reverse a string");
    println!("      • toUpper(text)      - Convert to uppercase");
    println!();
    println!("   👥 User Management:");
    println!("      • user.create(name, email, role) - Create a new user");
    println!("      • user.get(id)       - Get user by ID");
    println!("      • user.list()        - List all users");
    println!();
    println!("   🔧 Special Operations:");
    println!("      • slowOperation()    - Async operation (2s delay)");
    println!("      • testError()        - Always returns error (for testing)");
    println!();
    println!("   📨 Client Notifications (no response):");
    println!("      • log(message)       - Log a message to server console");
    println!("      • telemetry(data)    - Send telemetry/analytics data");
    println!("      • heartbeat(timestamp) - Send heartbeat ping");
    println!("      • activity(action, user) - Track user activity");
    println!();
    println!("   📡 Pub/Sub Topics (Subscribe in UI):");
    println!("      • server.stats       - Server statistics (every 5s)");
    println!("      • server.time        - Current time (every 10s)");
    println!("      • events.demo        - Demo events (every 15s)");
    println!("      • server.logs        - Server logs + RPC responses [Use Server Logs tab]");
    println!();
    println!("   💾 Persistent Subscription Topics:");
    println!("      • persistent.events       - Exactly-once delivery events (every 12s)");
    println!("      • persistent.notifications - Durable notifications (every 20s)");
    println!();
    println!("   ⚙️ Persistent Subscription Methods:");
    println!("      • rpc.subscribe_persistent(topic, subscription_id)");
    println!("      • rpc.ack_persistent(sequence_number)");
    println!("      • rpc.unsubscribe_persistent(subscription_id)");
    println!();
    println!("   📋 Server Logs includes:");
    println!("      • Real-time RPC request/response logs");
    println!("      • Client notification logs (log, telemetry, heartbeat, activity)");
    println!("      • Success and error responses");
    println!("      • Periodic server health/status messages");
    println!();
    println!("   💡 Features:");
    println!("      • Exactly-once delivery with acknowledgments");
    println!("      • Durable message storage (sled embedded database)");
    println!("      • Automatic redelivery on reconnect");
    println!("      • Message retention policies");
    println!();
    println!("Press Ctrl+C to stop...");
    println!();

    // Store server in Arc so we can share it across tasks
    let server = Arc::new(server);
    
    // Clone Arc for background publishing tasks
    let server_for_stats = Arc::clone(&server);
    let server_for_time = Arc::clone(&server);
    let server_for_events = Arc::clone(&server);
    let server_for_logs = Arc::clone(&server);
    let server_for_responses = Arc::clone(&server);
    let server_for_persistent_events = Arc::clone(&server);
    let server_for_persistent_notifications = Arc::clone(&server);

    // Spawn task to publish response logs from middleware
    tokio::spawn(async move {
        while let Some(log_data) = log_rx.recv().await {
            // Publish response log to server.logs topic
            match server_for_responses.publish("server.logs", log_data).await {
                Ok(count) if count > 0 => {
                    // Successfully published to subscribers
                }
                Ok(_) => {} // No subscribers
                Err(e) => {
                    eprintln!("Error publishing response log: {}", e);
                }
            }
        }
    });

    // Spawn task to publish server statistics periodically
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        let mut counter = 0;
        
        loop {
            interval.tick().await;
            counter += 1;
            
            let stats = serde_json::json!({
                "update_number": counter,
                "uptime_seconds": counter * 5,
                "memory_mb": 42 + (counter % 10), // Fake memory usage
                "connections": 1 + (counter % 3), // Fake connection count
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            // Publish to topic (subscribers will receive this)
            match server_for_stats.publish("server.stats", stats).await {
                Ok(count) if count > 0 => {
                    println!("📊 Published server stats to {} subscriber(s)", count);
                }
                Ok(_) => {} // No subscribers
                Err(e) => eprintln!("Error publishing stats: {}", e),
            }
        }
    });

    // Spawn task to publish current time periodically
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            let time_data = serde_json::json!({
                "utc": chrono::Utc::now().to_rfc3339(),
                "unix_timestamp": chrono::Utc::now().timestamp(),
                "timezone": "UTC"
            });

            match server_for_time.publish("server.time", time_data).await {
                Ok(count) if count > 0 => {
                    println!("⏰ Published time update to {} subscriber(s)", count);
                }
                Ok(_) => {} // No subscribers
                Err(e) => eprintln!("Error publishing time: {}", e),
            }
        }
    });

    // Spawn task to publish demo events
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(15));
        let events = vec![
            "User logged in",
            "Data synchronized",
            "Cache cleared",
            "Backup completed",
            "Task finished",
        ];
        let mut event_index = 0;
        
        loop {
            interval.tick().await;
            
            let event_data = serde_json::json!({
                "event": events[event_index],
                "severity": if event_index % 2 == 0 { "info" } else { "success" },
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            event_index = (event_index + 1) % events.len();

            match server_for_events.publish("events.demo", event_data).await {
                Ok(count) if count > 0 => {
                    println!("📢 Published demo event to {} subscriber(s)", count);
                }
                Ok(_) => {} // No subscribers
                Err(e) => eprintln!("Error publishing event: {}", e),
            }
        }
    });
    
    // Spawn task to publish server logs
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(8));
        let log_messages = vec![
            ("info", "Server health check passed"),
            ("success", "Background task completed successfully"),
            ("debug", "Processing scheduled maintenance"),
            ("info", "Connection pool status: healthy"),
            ("success", "Data backup completed"),
            ("debug", "Memory usage: 42MB / 512MB"),
            ("info", "Request queue: 0 pending"),
            ("success", "Cache sync completed"),
        ];
        let mut log_index = 0;
        
        // Wait a bit before starting
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        loop {
            interval.tick().await;
            
            let (level, message) = log_messages[log_index];
            
            let log_data = serde_json::json!({
                "level": level,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            log_index = (log_index + 1) % log_messages.len();

            // Publish using server.publish() which handles everything properly
            match server_for_logs.publish("server.logs", log_data.clone()).await {
                Ok(count) if count > 0 => {
                    println!("📋 Published server log to {} subscriber(s): {}", count, message);
                }
                Ok(_) => {} // No subscribers
                Err(e) => {
                    eprintln!("Error publishing log: {}", e);
                }
            }
        }
    });

    // Spawn task to publish persistent events
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(12));
        let mut event_id = 1;
        
        // Wait a bit before starting
        tokio::time::sleep(Duration::from_secs(7)).await;
        
        loop {
            interval.tick().await;
            
            let event_data = serde_json::json!({
                "event_id": event_id,
                "type": "persistent.event",
                "message": format!("Persistent event #{}", event_id),
                "priority": if event_id % 3 == 0 { "high" } else { "normal" },
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            
            event_id += 1;

            // Publish as persistent (will be stored in DB)
            match server_for_persistent_events.publish_persistent("persistent.events", event_data).await {
                Ok(seq) => {
                    println!("💾 Published persistent event (sequence: {})", seq);
                }
                Err(e) => {
                    eprintln!("Error publishing persistent event: {}", e);
                }
            }
        }
    });

    // Spawn task to publish persistent notifications
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(20));
        let notifications = vec![
            "System backup completed",
            "Security scan finished",
            "Data synchronization successful",
            "Health check passed",
            "Maintenance window scheduled",
        ];
        let mut notif_index = 0;
        
        // Wait a bit before starting
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        loop {
            interval.tick().await;
            
            let notification_data = serde_json::json!({
                "notification_id": notif_index + 1,
                "type": "persistent.notification",
                "message": notifications[notif_index],
                "severity": "info",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            
            notif_index = (notif_index + 1) % notifications.len();

            // Publish as persistent (will be stored in DB)
            match server_for_persistent_notifications.publish_persistent("persistent.notifications", notification_data).await {
                Ok(seq) => {
                    println!("💾 Published persistent notification (sequence: {})", seq);
                }
                Err(e) => {
                    eprintln!("Error publishing persistent notification: {}", e);
                }
            }
        }
    });

    // Run the server (Arc allows &self method access)
    server.run().await?;

    Ok(())
}

