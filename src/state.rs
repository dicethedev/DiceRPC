use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Represents a blockchain transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: u64,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Represents an account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

/// In-memory persistent state for the RPC server
///
/// This provides a simple key-value store for balances and transactions
/// In production, this would be backed by a real database
#[allow(dead_code)]
pub struct StateStore {
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    /// Get account by address, creating if it doesn't exist
    pub async fn get_or_create_account(&self, address: impl Into<String>) -> Account {
        let address = address.into();
        let mut accounts = self.accounts.write().await;

        accounts
            .entry(address.clone())
            .or_insert_with(|| Account {
                address: address.clone(),
                balance: 0,
                nonce: 0,
            })
            .clone()
    }

    #[allow(dead_code)]
    /// Get account balance
    pub async fn get_balance(&self, address: &str) -> Option<u64> {
        self.accounts
            .read()
            .await
            .get(address)
            .map(|acc| acc.balance)
    }

    #[allow(dead_code)]
    /// Update account balance
    pub async fn set_balance(&self, address: impl Into<String>, balance: u64) {
        let address = address.into();
        let mut accounts = self.accounts.write().await;

        accounts
            .entry(address.clone())
            .and_modify(|acc| acc.balance = balance)
            .or_insert(Account {
                address,
                balance,
                nonce: 0,
            });
    }

    #[allow(dead_code)]
    /// Transfer funds between accounts
    pub async fn transfer(&self, from: &str, to: &str, amount: u64) -> Result<Transaction, String> {
        let mut accounts = self.accounts.write().await;

        let sender_balance = accounts
            .get(from)
            .ok_or_else(|| "Sender account not found".to_string())?
            .balance;

        if sender_balance < amount {
            return Err("Insufficient balance".to_string());
        }

        if from == to {
            let sender = accounts
                .get_mut(from)
                .ok_or_else(|| "Sender account not found".to_string())?;
            sender.nonce = sender
                .nonce
                .checked_add(1)
                .ok_or_else(|| "Sender nonce overflow".to_string())?;
        } else {
            let receiver_balance = accounts.get(to).map_or(0, |account| account.balance);
            let new_receiver_balance = receiver_balance
                .checked_add(amount)
                .ok_or_else(|| "Receiver balance overflow".to_string())?;

            let sender = accounts
                .get_mut(from)
                .ok_or_else(|| "Sender account not found".to_string())?;
            sender.balance = sender_balance - amount;
            sender.nonce = sender
                .nonce
                .checked_add(1)
                .ok_or_else(|| "Sender nonce overflow".to_string())?;

            accounts
                .entry(to.to_string())
                .and_modify(|account| account.balance = new_receiver_balance)
                .or_insert(Account {
                    address: to.to_string(),
                    balance: new_receiver_balance,
                    nonce: 0,
                });
        }

        // Create transaction record
        let tx = Transaction {
            txid: Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            status: TransactionStatus::Pending,
        };

        // Store transaction
        self.transactions
            .write()
            .await
            .insert(tx.txid.clone(), tx.clone());

        Ok(tx)
    }

    #[allow(dead_code)]
    /// Get transaction by ID
    pub async fn get_transaction(&self, txid: &str) -> Option<Transaction> {
        self.transactions.read().await.get(txid).cloned()
    }

    #[allow(dead_code)]
    /// Confirm a pending transaction
    pub async fn confirm_transaction(&self, txid: &str) -> Result<(), String> {
        let mut transactions = self.transactions.write().await;
        let tx = transactions
            .get_mut(txid)
            .ok_or_else(|| "Transaction not found".to_string())?;

        tx.status = TransactionStatus::Confirmed;
        Ok(())
    }

    #[allow(dead_code)]
    /// Get all transactions for an address
    pub async fn get_transactions_for_address(&self, address: &str) -> Vec<Transaction> {
        self.transactions
            .read()
            .await
            .values()
            .filter(|tx| tx.from == address || tx.to == address)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    /// Get all accounts
    pub async fn get_all_accounts(&self) -> Vec<Account> {
        self.accounts.read().await.values().cloned().collect()
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}
