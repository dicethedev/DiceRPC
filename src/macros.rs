/// Macros for ergonomic RPC method registration
/// 
/// Usage:
/// ```rust
/// rpc_handler!(my_method, params => {
///     let value = params["key"].as_str().ok_or_invalid_params()?;
///     Ok(json!({"result": value}))
/// });
/// 
/// server.register_handler("my_method", my_method).await;
/// ```
/// 

#[macro_export]
macro_rules! rpc_handler {
    ($name:ident, $params:ident => $body:block) => {
        pub fn $name(
            $params: serde_json::Value,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<serde_json::Value, $crate::rpc::RpcErrorObj>>
                    + Send,
            >,
        > {
            Box::pin(async move { $body })
        }
    };
}

/// Macro to register multiple handlers at once
/// 
/// Usage:
/// ```rust
/// register_handlers!(server, {
///     "ping" => ping_handler,
///     "get_balance" => get_balance_handler,
///     "send_tx" => send_tx_handler,
/// });
/// ```
#[macro_export]
macro_rules! register_handlers {
    ($server:expr, { $($method:expr => $handler:expr),* $(,)? }) => {
        {
            $(
                $server.register($method, $handler).await;
            )*
        }
    };
}

/// Helper trait to convert Option to RpcErrorObj
#[allow(dead_code)]
pub trait OptionExt<T> {
    fn ok_or_invalid_params(self) -> Result<T, crate::rpc::RpcErrorObj>;
    fn ok_or_rpc_error(self, code: i64, msg: impl Into<String>) -> Result<T, crate::rpc::RpcErrorObj>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_invalid_params(self) -> Result<T, crate::rpc::RpcErrorObj> {
        self.ok_or(crate::rpc::RpcErrorObj {
            code: crate::rpc::INVALID_PARAMS,
            message: "Invalid parameters".to_string(),
            data: None,
        })
    }

    fn ok_or_rpc_error(self, code: i64, msg: impl Into<String>) -> Result<T, crate::rpc::RpcErrorObj> {
        self.ok_or(crate::rpc::RpcErrorObj {
            code,
            message: msg.into(),
            data: None,
        })
    }
}

// Example usage in your handlers file:
// 
// ```rust
// use dice_rpc::*;
// use serde_json::json;
// 
// rpc_handler!(ping, _params => {
//     Ok(json!("pong"))
// });
// 
// rpc_handler!(get_balance, params => {
//     let address = params["address"]
//         .as_str()
//         .ok_or_invalid_params()?;
//     
//     let balance = fetch_balance(address).await?;
//     Ok(json!({"balance": balance}))
// });
// 
// pub async fn setup_handlers(server: &RpcServer) {
//     register_handlers!(server, {
//         "ping" => ping,
//         "get_balance" => get_balance,
//     });
// }
// ```
