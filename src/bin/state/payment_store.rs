use std::{collections::HashMap, fmt};

use parking_lot::Mutex;

/// A payment session awaiting user confirmation or cancellation.
#[derive(Debug, Clone)]
pub struct PaymentSession {
    pub payment_id: String,
    pub profile_id: i64,
    #[expect(dead_code)]
    pub token: String,
    pub sku_id: i64,
    pub price: i64,
    pub count: i64,
    pub name: String,
    pub description: String,
}

/// In-memory store for payment sessions.
pub struct PaymentStore {
    sessions: Mutex<HashMap<String, PaymentSession>>,
}

impl PaymentStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Insert a session.
    pub fn insert(&self, session: PaymentSession) {
        self.sessions.lock().insert(session.payment_id.clone(), session);
    }

    /// Get a session (non-consuming clone).
    pub fn get(&self, payment_id: &str) -> Option<PaymentSession> {
        self.sessions.lock().get(payment_id).cloned()
    }

    /// Take a session (consuming remove).
    pub fn take(&self, payment_id: &str) -> Option<PaymentSession> {
        self.sessions.lock().remove(payment_id)
    }
}

impl Default for PaymentStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for PaymentStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaymentStore").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(id: &str) -> PaymentSession {
        PaymentSession {
            payment_id: id.to_string(),
            profile_id: 1,
            token: "test-token".to_string(),
            sku_id: 100,
            price: 500,
            count: 2,
            name: "Test Item".to_string(),
            description: "A test item".to_string(),
        }
    }

    #[test]
    fn insert_then_get_returns_identical_session() {
        let store = PaymentStore::new();
        let session = test_session("abc-123");
        store.insert(session.clone());
        let got = store.get("abc-123").unwrap();
        assert_eq!(got.payment_id, session.payment_id);
        assert_eq!(got.sku_id, session.sku_id);
    }

    #[test]
    fn insert_then_take_returns_session_and_removes_it() {
        let store = PaymentStore::new();
        store.insert(test_session("abc-123"));
        let taken = store.take("abc-123").unwrap();
        assert_eq!(taken.payment_id, "abc-123");
        assert!(store.get("abc-123").is_none());
    }

    #[test]
    fn get_after_take_returns_none() {
        let store = PaymentStore::new();
        store.insert(test_session("abc-123"));
        store.take("abc-123");
        assert!(store.get("abc-123").is_none());
    }

    #[test]
    fn take_on_nonexistent_returns_none() {
        let store = PaymentStore::new();
        assert!(store.take("nonexistent").is_none());
    }
}
