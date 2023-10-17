use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::amount::Amount;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub id: u8,
    pub timestamp: DateTime<Utc>,
    pub child_name: String,
    pub amount: Amount,
    pub purpose: String,
}

impl Transaction {
    pub fn new(
        id: u8,
        timestamp: DateTime<Utc>,
        child_name: String,
        amount: Amount,
        purpose: String,
    ) -> Transaction {
        Transaction {
            id,
            timestamp,
            child_name,
            amount,
            purpose,
        }
    }
}
