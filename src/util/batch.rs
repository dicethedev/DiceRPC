use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::rpc::{RpcRequest, RpcResponse, RpcServer};

/// Represents either a single request or a batch of requests
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BatchRequest {
    Single(RpcRequest),
    Batch(Vec<RpcRequest>),
}

/// Represents either a single response or a batch of responses
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BatchResponse {
    Single(RpcResponse),
    Batch(Vec<RpcResponse>),
}

impl BatchRequest {
    #[allow(dead_code)]
    /// Parse raw JSON string into a BatchRequest
    pub fn parse(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    #[allow(dead_code)]
    /// Check if this is a batch request
    pub fn is_batch(&self) -> bool {
        matches!(self, BatchRequest::Batch(_))
    }

    /// Get the number of requests in this batch
    pub fn len(&self) -> usize {
        match self {
            BatchRequest::Single(_) => 1,
            BatchRequest::Batch(v) => v.len(),
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RpcServer {
    #[allow(dead_code)]
    /// Handle a batch request by processing all requests concurrently
    pub async fn handle_batch(&self, batch: BatchRequest) -> BatchResponse {
        match batch {
            BatchRequest::Single(req) => {
                BatchResponse::Single(self.handle_request(req).await)
            }
            BatchRequest::Batch(requests) => {
                if requests.is_empty() {
                    // Empty batch is invalid
                    return BatchResponse::Single(RpcResponse::with_error(
                        Value::Null,
                        -32600,
                        "Invalid Request: empty batch",
                    ));
                }

                // Process all requests concurrently
                let futures: Vec<_> = requests
                    .into_iter()
                    .map(|req| self.handle_request(req))
                    .collect();

                let responses = futures::future::join_all(futures).await;
                BatchResponse::Batch(responses)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_single_request() {
        let raw = r#"{"jsonrpc":"2.0","method":"ping","params":{},"id":1}"#;
        let batch = BatchRequest::parse(raw).unwrap();
        assert!(!batch.is_batch());
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_parse_batch_request() {
        let raw = r#"[
            {"jsonrpc":"2.0","method":"ping","params":{},"id":1},
            {"jsonrpc":"2.0","method":"get_balance","params":{"address":"0x123"},"id":2}
        ]"#;
        let batch = BatchRequest::parse(raw).unwrap();
        assert!(batch.is_batch());
        assert_eq!(batch.len(), 2);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        use crate::rpc::RpcServer;

        let server = RpcServer::new();
        
        // Register a simple handler
        server.register("ping", |_| async move {
            Ok(json!("pong"))
        }).await;

        // Create batch request
        let requests = vec![
            RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "ping".to_string(),
                params: json!({}),
                id: json!(1),
            },
            RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "ping".to_string(),
                params: json!({}),
                id: json!(2),
            },
        ];

        let batch = BatchRequest::Batch(requests);
        let response = server.handle_batch(batch).await;

        match response {
            BatchResponse::Batch(responses) => {
                assert_eq!(responses.len(), 2);
                assert_eq!(responses[0].result, Some(json!("pong")));
                assert_eq!(responses[1].result, Some(json!("pong")));
            }
            _ => panic!("Expected batch response"),
        }
    }
}