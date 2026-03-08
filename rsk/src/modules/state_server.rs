use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateRequest {
    Put { key: String, value: Value },
    Get { key: String },
    Delete { key: String },
    ListKeys,
    Stats,
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateResponse {
    Ok {
        value: Option<Value>,
    },
    Keys {
        keys: Vec<String>,
    },
    Stats {
        count: usize,
        memory_estimate: usize,
    },
    Error {
        message: String,
    },
}

pub struct StateStore {
    data: DashMap<String, Value>,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }
    pub fn put(&self, key: String, value: Value) {
        self.data.insert(key, value);
    }
    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).map(|v| v.value().clone())
    }
    pub fn delete(&self, key: &str) -> bool {
        self.data.remove(key).is_some()
    }
    pub fn list_keys(&self) -> Vec<String> {
        self.data.iter().map(|r| r.key().clone()).collect()
    }
    pub fn stats(&self) -> (usize, usize) {
        let count = self.data.len();
        let mem = self
            .data
            .iter()
            .map(|r| r.key().len() + serde_json::to_string(r.value()).unwrap_or_default().len())
            .sum();
        (count, mem)
    }
}

pub struct StateServer {
    store: Arc<StateStore>,
    socket_path: String,
}

impl StateServer {
    pub fn new(socket_path: &str) -> Self {
        Self {
            store: Arc::new(StateStore::new()),
            socket_path: socket_path.to_string(),
        }
    }
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // PROOF: Socket Takeover (Phase 2 Substance)
        if Path::new(&self.socket_path).exists() {
            eprintln!(
                "[server] Socket path exists, unlinking: {}",
                self.socket_path
            );
            fs::remove_file(&self.socket_path).map_err(|e| {
                format!(
                    "Failed to unlink existing socket {}: {}",
                    self.socket_path, e
                )
            })?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        fs::set_permissions(&self.socket_path, fs::Permissions::from_mode(0o666)).ok();

        println!("[server] State server listening on: {}", self.socket_path);

        loop {
            let (mut stream, _) = listener.accept().await?;
            let store = Arc::clone(&self.store);
            tokio::spawn(async move {
                let mut buffer = [0; 8192];
                while let Ok(n) = stream.read(&mut buffer).await {
                    if n == 0 {
                        break;
                    }
                    if let Ok(request) = serde_json::from_slice::<StateRequest>(&buffer[..n]) {
                        let response = match request {
                            StateRequest::Put { key, value } => {
                                store.put(key, value);
                                StateResponse::Ok { value: None }
                            }
                            StateRequest::Get { key } => StateResponse::Ok {
                                value: store.get(&key),
                            },
                            StateRequest::Delete { key } => StateResponse::Ok {
                                value: Some(Value::Bool(store.delete(&key))),
                            },
                            StateRequest::ListKeys => StateResponse::Keys {
                                keys: store.list_keys(),
                            },
                            StateRequest::Stats => {
                                let (count, memory_estimate) = store.stats();
                                StateResponse::Stats {
                                    count,
                                    memory_estimate,
                                }
                            }
                            StateRequest::Shutdown => std::process::exit(0),
                        };

                        if let Ok(resp_bytes) = serde_json::to_vec(&response) {
                            let _ = stream.write_all(&resp_bytes).await;
                        }
                    }
                }
            });
        }
    }
}
