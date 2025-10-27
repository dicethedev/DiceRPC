use dice_rpc::state::{StateStore, TransactionStatus};

#[tokio::test]
async fn test_get_or_create_account() {
    let store = StateStore::new();
    let account = store.get_or_create_account("0x123").await;

    assert_eq!(account.address, "0x123");
    assert_eq!(account.balance, 0);
    assert_eq!(account.nonce, 0);
}

#[tokio::test]
async fn test_balance_operations() {
    let store = StateStore::new();

    // Set balance
    store.set_balance("0x123", 1000).await;

    // Get balance
    let balance = store.get_balance("0x123").await;
    assert_eq!(balance, Some(1000));
}

#[tokio::test]
async fn test_transfer() {
    let store = StateStore::new();

    // Setup accounts
    store.set_balance("0x123", 1000).await;
    store.set_balance("0x456", 500).await;

    // Transfer
    let tx = store.transfer("0x123", "0x456", 300).await.unwrap();

    // Check balances
    assert_eq!(store.get_balance("0x123").await, Some(700));
    assert_eq!(store.get_balance("0x456").await, Some(800));

    // Check transaction
    assert_eq!(tx.from, "0x123");
    assert_eq!(tx.to, "0x456");
    assert_eq!(tx.amount, 300);
    assert_eq!(tx.status, TransactionStatus::Pending);
}

#[tokio::test]
async fn test_insufficient_balance() {
    let store = StateStore::new();
    store.set_balance("0x123", 100).await;

    let result = store.transfer("0x123", "0x456", 200).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Insufficient balance");
}

#[tokio::test]
async fn test_transaction_confirmation() {
    let store = StateStore::new();
    store.set_balance("0x123", 1000).await;

    let tx = store.transfer("0x123", "0x456", 100).await.unwrap();

    // Confirm transaction
    store.confirm_transaction(&tx.txid).await.unwrap();

    // Check status
    let confirmed_tx = store.get_transaction(&tx.txid).await.unwrap();
    assert_eq!(confirmed_tx.status, TransactionStatus::Confirmed);
}

#[tokio::test]
async fn test_get_transactions_for_address() {
    let store = StateStore::new();

    // Setup accounts
    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;
    store.set_balance("0xCarol", 300).await;

    // Alice sends to Bob
    let tx1 = store.transfer("0xAlice", "0xBob", 100).await.unwrap();

    // Bob sends to Carol
    let tx2 = store.transfer("0xBob", "0xCarol", 50).await.unwrap();

    // Alice sends to Carol
    let tx3 = store.transfer("0xAlice", "0xCarol", 200).await.unwrap();

    // Get Alice's transactions (should have 2: sent to Bob and Carol)
    let alice_txs = store.get_transactions_for_address("0xAlice").await;
    assert_eq!(alice_txs.len(), 2);
    assert!(alice_txs.iter().any(|tx| tx.txid == tx1.txid));
    assert!(alice_txs.iter().any(|tx| tx.txid == tx3.txid));

    // Get Bob's transactions (should have 2: received from Alice, sent to Carol)
    let bob_txs = store.get_transactions_for_address("0xBob").await;
    assert_eq!(bob_txs.len(), 2);
    assert!(bob_txs.iter().any(|tx| tx.txid == tx1.txid));
    assert!(bob_txs.iter().any(|tx| tx.txid == tx2.txid));

    // Get Carol's transactions (should have 2: received from Bob and Alice)
    let carol_txs = store.get_transactions_for_address("0xCarol").await;
    assert_eq!(carol_txs.len(), 2);
    assert!(carol_txs.iter().any(|tx| tx.txid == tx2.txid));
    assert!(carol_txs.iter().any(|tx| tx.txid == tx3.txid));
}

#[tokio::test]
async fn test_get_transactions_for_address_empty() {
    let store = StateStore::new();

    // Get transactions for address with no transactions
    let txs = store.get_transactions_for_address("0xNobody").await;
    assert_eq!(txs.len(), 0);
}

#[tokio::test]
async fn test_get_all_accounts() {
    let store = StateStore::new();

    // Initially empty
    let accounts = store.get_all_accounts().await;
    assert_eq!(accounts.len(), 0);

    // Add some accounts
    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;
    store.set_balance("0xCarol", 300).await;

    // Should have 3 accounts
    let accounts = store.get_all_accounts().await;
    assert_eq!(accounts.len(), 3);

    // Check all accounts exist
    assert!(
        accounts
            .iter()
            .any(|a| a.address == "0xAlice" && a.balance == 1000)
    );
    assert!(
        accounts
            .iter()
            .any(|a| a.address == "0xBob" && a.balance == 500)
    );
    assert!(
        accounts
            .iter()
            .any(|a| a.address == "0xCarol" && a.balance == 300)
    );
}

#[tokio::test]
async fn test_get_all_accounts_after_transfers() {
    let store = StateStore::new();

    // Setup and do transfers
    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;
    store.transfer("0xAlice", "0xBob", 200).await.unwrap();

    // Get all accounts
    let accounts = store.get_all_accounts().await;
    assert_eq!(accounts.len(), 2);

    // Check balances updated correctly
    let alice = accounts.iter().find(|a| a.address == "0xAlice").unwrap();
    let bob = accounts.iter().find(|a| a.address == "0xBob").unwrap();

    assert_eq!(alice.balance, 800);
    assert_eq!(bob.balance, 700);
    assert_eq!(alice.nonce, 1); // Nonce should increment
    assert_eq!(bob.nonce, 0);
}

#[tokio::test]
async fn test_nonce_increments_on_transfer() {
    let store = StateStore::new();

    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;

    // Do multiple transfers
    store.transfer("0xAlice", "0xBob", 100).await.unwrap();
    store.transfer("0xAlice", "0xBob", 100).await.unwrap();
    store.transfer("0xAlice", "0xBob", 100).await.unwrap();

    // Check Alice's nonce incremented
    let alice = store.get_or_create_account("0xAlice").await;
    assert_eq!(alice.nonce, 3);

    // Check Bob's nonce didn't increment (he only received)
    let bob = store.get_or_create_account("0xBob").await;
    assert_eq!(bob.nonce, 0);
}

#[tokio::test]
async fn test_get_transaction_by_id() {
    let store = StateStore::new();

    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;

    let tx = store.transfer("0xAlice", "0xBob", 100).await.unwrap();

    // Get transaction by ID
    let retrieved = store.get_transaction(&tx.txid).await;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.txid, tx.txid);
    assert_eq!(retrieved.from, "0xAlice");
    assert_eq!(retrieved.to, "0xBob");
    assert_eq!(retrieved.amount, 100);
    assert_eq!(retrieved.status, TransactionStatus::Pending);

    // Try to get non-existent transaction
    let non_existent = store.get_transaction("fake-txid-123").await;
    assert!(non_existent.is_none());
}

#[tokio::test]
async fn test_transfer_to_new_account() {
    let store = StateStore::new();

    // Only create sender account
    store.set_balance("0xAlice", 1000).await;

    // Transfer to non-existent account
    let tx = store
        .transfer("0xAlice", "0xNewAccount", 300)
        .await
        .unwrap();

    // Check sender balance
    assert_eq!(store.get_balance("0xAlice").await, Some(700));

    // Check receiver account was created with correct balance
    assert_eq!(store.get_balance("0xNewAccount").await, Some(300));

    // Verify the new account exists
    let new_account = store.get_or_create_account("0xNewAccount").await;
    assert_eq!(new_account.balance, 300);
    assert_eq!(new_account.nonce, 0);
}

#[tokio::test]
async fn test_transfer_from_non_existent_account() {
    let store = StateStore::new();

    store.set_balance("0xBob", 500).await;

    // Try to transfer from account that doesn't exist
    let result = store.transfer("0xNonExistent", "0xBob", 100).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Sender account not found");
}

#[tokio::test]
async fn test_confirm_non_existent_transaction() {
    let store = StateStore::new();

    let result = store.confirm_transaction("fake-txid").await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Transaction not found");
}

#[tokio::test]
async fn test_multiple_transaction_confirmations() {
    let store = StateStore::new();

    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;

    // Create multiple transactions
    let tx1 = store.transfer("0xAlice", "0xBob", 100).await.unwrap();
    let tx2 = store.transfer("0xAlice", "0xBob", 200).await.unwrap();

    // Confirm first transaction
    store.confirm_transaction(&tx1.txid).await.unwrap();

    // Check status
    let confirmed_tx1 = store.get_transaction(&tx1.txid).await.unwrap();
    assert_eq!(confirmed_tx1.status, TransactionStatus::Confirmed);

    let pending_tx2 = store.get_transaction(&tx2.txid).await.unwrap();
    assert_eq!(pending_tx2.status, TransactionStatus::Pending);

    // Confirm second transaction
    store.confirm_transaction(&tx2.txid).await.unwrap();

    let confirmed_tx2 = store.get_transaction(&tx2.txid).await.unwrap();
    assert_eq!(confirmed_tx2.status, TransactionStatus::Confirmed);
}

#[tokio::test]
async fn test_transaction_timestamp() {
    let store = StateStore::new();

    store.set_balance("0xAlice", 1000).await;
    store.set_balance("0xBob", 500).await;

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let tx = store.transfer("0xAlice", "0xBob", 100).await.unwrap();

    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Transaction timestamp should be between before and after
    assert!(tx.timestamp >= before);
    assert!(tx.timestamp <= after);
}

#[tokio::test]
async fn test_complex_transaction_flow() {
    let store = StateStore::new();

    // Setup accounts
    store.set_balance("0xAlice", 10000).await;
    store.set_balance("0xBob", 5000).await;
    store.set_balance("0xCarol", 2000).await;

    // Create a series of transactions
    let tx1 = store.transfer("0xAlice", "0xBob", 1000).await.unwrap();
    let tx2 = store.transfer("0xBob", "0xCarol", 500).await.unwrap();
    let tx3 = store.transfer("0xCarol", "0xAlice", 300).await.unwrap();
    let tx4 = store.transfer("0xAlice", "0xCarol", 200).await.unwrap();

    // Confirm some transactions
    store.confirm_transaction(&tx1.txid).await.unwrap();
    store.confirm_transaction(&tx3.txid).await.unwrap();

    // Check final balances
    assert_eq!(store.get_balance("0xAlice").await, Some(9100)); // 10000 - 1000 + 300 - 200
    assert_eq!(store.get_balance("0xBob").await, Some(5500)); // 5000 + 1000 - 500
    assert_eq!(store.get_balance("0xCarol").await, Some(2400)); // 2000 + 500 - 300 + 200

    // Check transaction statuses
    assert_eq!(
        store.get_transaction(&tx1.txid).await.unwrap().status,
        TransactionStatus::Confirmed
    );
    assert_eq!(
        store.get_transaction(&tx2.txid).await.unwrap().status,
        TransactionStatus::Pending
    );
    assert_eq!(
        store.get_transaction(&tx3.txid).await.unwrap().status,
        TransactionStatus::Confirmed
    );
    assert_eq!(
        store.get_transaction(&tx4.txid).await.unwrap().status,
        TransactionStatus::Pending
    );

    // Check transaction counts per address
    assert_eq!(store.get_transactions_for_address("0xAlice").await.len(), 3);
    assert_eq!(store.get_transactions_for_address("0xBob").await.len(), 2);
    assert_eq!(store.get_transactions_for_address("0xCarol").await.len(), 3);

    // Check nonces
    let alice = store.get_or_create_account("0xAlice").await;
    let bob = store.get_or_create_account("0xBob").await;
    let carol = store.get_or_create_account("0xCarol").await;

    assert_eq!(alice.nonce, 2); // Sent twice
    assert_eq!(bob.nonce, 1); // Sent once
    assert_eq!(carol.nonce, 1); // Sent once
}
