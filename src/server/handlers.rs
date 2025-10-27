use crate::rpc::{RpcServer, RpcErrorObj, INVALID_PARAMS};
use crate::state::{StateStore, TransactionStatus};
use serde_json::{json, Value};
use std::sync::Arc;

#[allow(dead_code)]
/// Register handlers with persistent state
pub async fn register_stateful_handlers(server: &RpcServer, state: Arc<StateStore>) {
    // Ping handler - simple health check
    server
        .register("ping", |_params| async move { Ok(Value::String("pong".into())) })
        .await;

    // Get balance - now uses real state
    {
        let state = state.clone();
        server
            .register("get_balance", move |params| {
                let state = state.clone();
                async move {
                    let address = params
                        .get("address")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'address' parameter".into(),
                            data: None,
                        })?;

                    let balance = state.get_balance(address).await.unwrap_or(0);

                    Ok(json!({
                        "address": address,
                        "balance": balance.to_string()
                    }))
                }
            })
            .await;
    }

    // Set balance - admin function for testing
    {
        let state = state.clone();
        server
            .register("set_balance", move |params| {
                let state = state.clone();
                async move {
                    let address = params
                        .get("address")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'address' parameter".into(),
                            data: None,
                        })?;

                    let balance = params
                        .get("balance")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing or invalid 'balance' parameter".into(),
                            data: None,
                        })?;

                    state.set_balance(address, balance).await;

                    Ok(json!({
                        "address": address,
                        "balance": balance.to_string(),
                        "success": true
                    }))
                }
            })
            .await;
    }

    // Transfer - send funds between accounts
    {
        let state = state.clone();
        server
            .register("transfer", move |params| {
                let state = state.clone();
                async move {
                    let from = params
                        .get("from")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'from' parameter".into(),
                            data: None,
                        })?;

                    let to = params
                        .get("to")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'to' parameter".into(),
                            data: None,
                        })?;

                    let amount = params
                        .get("amount")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing or invalid 'amount' parameter".into(),
                            data: None,
                        })?;

                    match state.transfer(from, to, amount).await {
                        Ok(tx) => Ok(json!({
                            "txid": tx.txid,
                            "from": tx.from,
                            "to": tx.to,
                            "amount": tx.amount,
                            "status": "pending"
                        })),
                        Err(e) => Err(RpcErrorObj {
                            code: -32000,
                            message: e,
                            data: None,
                        }),
                    }
                }
            })
            .await;
    }

    // Get transaction by ID
    {
        let state = state.clone();
        server
            .register("get_transaction", move |params| {
                let state = state.clone();
                async move {
                    let txid = params
                        .get("txid")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'txid' parameter".into(),
                            data: None,
                        })?;

                    match state.get_transaction(txid).await {
                        Some(tx) => Ok(json!({
                            "txid": tx.txid,
                            "from": tx.from,
                            "to": tx.to,
                            "amount": tx.amount,
                            "timestamp": tx.timestamp,
                            "status": match tx.status {
                               TransactionStatus::Pending => "pending",
                               TransactionStatus::Confirmed => "confirmed",
                               TransactionStatus::Failed => "failed",
                            }
                        })),
                        None => Err(RpcErrorObj {
                            code: -32001,
                            message: "Transaction not found".into(),
                            data: None,
                        }),
                    }
                }
            })
            .await;
    }

    // Confirm transaction
    {
        let state = state.clone();
        server
            .register("confirm_transaction", move |params| {
                let state = state.clone();
                async move {
                    let txid = params
                        .get("txid")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'txid' parameter".into(),
                            data: None,
                        })?;

                    match state.confirm_transaction(txid).await {
                        Ok(_) => Ok(json!({
                            "txid": txid,
                            "status": "confirmed",
                            "success": true
                        })),
                        Err(e) => Err(RpcErrorObj {
                            code: -32001,
                            message: e,
                            data: None,
                        }),
                    }
                }
            })
            .await;
    }

    // Get transactions for address
    {
        let state = state.clone();
        server
            .register("get_transactions", move |params| {
                let state = state.clone();
                async move {
                    let address = params
                        .get("address")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcErrorObj {
                            code: INVALID_PARAMS,
                            message: "Missing 'address' parameter".into(),
                            data: None,
                        })?;

                    let transactions = state.get_transactions_for_address(address).await;

                    let tx_list: Vec<Value> = transactions
                        .iter()
                        .map(|tx| {
                            json!({
                                "txid": tx.txid,
                                "from": tx.from,
                                "to": tx.to,
                                "amount": tx.amount,
                                "timestamp": tx.timestamp,
                                "status": match tx.status {
                                    TransactionStatus::Pending => "pending",
                                    TransactionStatus::Confirmed => "confirmed",
                                    TransactionStatus::Failed => "failed",
                                }
                            })
                        })
                        .collect();

                    Ok(json!({
                        "address": address,
                        "transactions": tx_list
                    }))
                }
            })
            .await;
    }

    // List all accounts
    {
        let state = state.clone();
        server
            .register("list_accounts", move |_params| {
                let state = state.clone();
                async move {
                    let accounts = state.get_all_accounts().await;

                    let acc_list: Vec<Value> = accounts
                        .iter()
                        .map(|acc| {
                            json!({
                                "address": acc.address,
                                "balance": acc.balance.to_string(),
                                "nonce": acc.nonce
                            })
                        })
                        .collect();

                    Ok(json!({
                        "accounts": acc_list,
                        "count": acc_list.len()
                    }))
                }
            })
            .await;
    }
}