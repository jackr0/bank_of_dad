use std::sync::{Mutex, MutexGuard};

use chrono::{TimeZone, Utc};

use rusqlite::{params, Connection};

use crate::model::{amount::Amount, error::ApiError, transaction::Transaction};

pub struct Db {
    connection: Mutex<Connection>,
}

impl Db {
    pub fn new() -> Db {
        let connection = Connection::open_in_memory().unwrap();

        connection
            .execute(
                "CREATE TABLE transactions (
                    id INTEGER PRIMARY KEY,
                    timestamp INTEGER NOT NULL,
                    child_name TEXT NOT NULL,
                    amount INTEGER NOT NULL,
                    purpose TEXT NOT NULL
                )",
                (),
            )
            .unwrap();

        Db {
            connection: Mutex::new(connection),
        }
    }

    pub fn record_transaction_for_child(
        &self,
        transaction: Transaction,
    ) -> Result<Transaction, ApiError> {
        let conn = self.connection.lock().unwrap();

        if transaction.amount.is_negative() {
            let new_balance = Self::get_account_balance_for_child_internal(
                &conn,
                transaction.child_name.clone(),
            )? + transaction.amount;

            if new_balance.is_negative() {
                return Err(ApiError::InputFailedValidation(format!(
                    "Transaction will take account {} negative",
                    transaction.child_name
                )));
            }
        }

        conn.execute(
            "INSERT INTO transactions (timestamp, child_name, amount, purpose) VALUES (?1, ?2, ?3, ?4)",
            params![
                transaction.timestamp.timestamp_millis(),
                transaction.child_name,
                transaction.amount.serialize_for_db(),
                transaction.purpose
            ],
        )?;

        let transaction_id = conn.last_insert_rowid();
        let transaction_result = Transaction {
            id: u8::try_from(transaction_id).unwrap(),
            ..transaction
        };

        Ok(transaction_result)
    }

    pub fn get_transactions_for_child(
        &self,
        child_name: String,
    ) -> Result<Vec<Transaction>, ApiError> {
        let conn = self.connection.lock().unwrap();
        let mut transactions: Vec<Transaction> = Vec::new();

        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, child_name, amount, purpose FROM transactions WHERE child_name = ?1",
            )?;

        let mut rows = stmt.query(params![child_name]).unwrap();
        while let Ok(Some(row)) = rows.next() {
            let timestamp = Utc
                .timestamp_millis_opt(row.get::<usize, i64>(1)?)
                .single()
                .unwrap();

            transactions.push(Transaction::new(
                row.get::<usize, u8>(0)?,
                timestamp,
                row.get::<usize, String>(2)?,
                Amount::deserialize_from_db(row.get::<usize, i64>(3)?),
                row.get::<usize, String>(4)?,
            ))
        }

        return Ok(transactions);
    }

    pub fn get_account_balance_for_child(&self, child_name: String) -> Result<Amount, ApiError> {
        let conn = self.connection.lock().unwrap();
        Self::get_account_balance_for_child_internal(&conn, child_name)
    }

    fn get_account_balance_for_child_internal(
        conn: &MutexGuard<'_, Connection>,
        child_name: String,
    ) -> Result<Amount, ApiError> {
        let balance_amount: Amount = conn.query_row(
            "SELECT COALESCE(sum(amount),0) AS amount FROM transactions WHERE child_name = ?1",
            params![child_name],
            |r| Ok(Amount::deserialize_from_db(r.get::<usize, i64>(0)?)),
        )?;

        return Ok(balance_amount);
    }
}
