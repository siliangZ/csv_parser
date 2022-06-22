use csv::{Reader};

use snafu::Snafu;
use std::{
    collections::HashMap,
    io::{Read, Write},
};
mod client;
mod transaction;
pub use client::{ClientAccount, ClientAccountInfo};
pub use transaction::{Amount, ClientID, Transaction, TransactionID, TransactionType};

// the error type used in the program
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("reach to the wrong account"))]
    WrongAccount,
    #[snafu(display(
        "no sufficient funds in client {}\ncurrent available {}, try to withdrawal {}",
        client,
        available,
        withdrawal
    ))]
    NoSufficientFunds {
        client: ClientID,
        available: Amount,
        withdrawal: Amount,
    },
    #[snafu(display(
        "can't process transaction for client {}, because the account is locked",
        client
    ))]
    AccountLocked { client: ClientID },
    #[snafu(display("can't find transaction {} in client {} account", tx, client))]
    NotFoundTransaction { client: ClientID, tx: TransactionID },
    #[snafu(display("the transaction:\nclient: {}\ntx: {}\ntype: {:?}\namount: {:?}\n is not valid, please check the record", client, tx, t_type, amount))]
    InvalidTransaction {
        client: ClientID,
        tx: TransactionID,
        t_type: TransactionType,
        amount: Option<Amount>,
    },
}

pub fn build_csv_reader<R: Read>(stream_reader: R) -> Reader<R> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .flexible(true)
        .terminator(csv::Terminator::Any(b'\n')) // specify the terminator
        .trim(csv::Trim::All)
        .from_reader(stream_reader)
}

// the structure that holds the record of transactions
// it keeps the pair of <TransactionID, (ClientID, Amount)>
// the ClientID here is not necessary since it is used to prevent the dispute with wrong clientID(which may not happen :)
pub struct TransactionHashmapDB {
    transactions: HashMap<TransactionID, (ClientID, Amount)>,
}
impl TransactionHashmapDB {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }

    // add a transaction into the record
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        match transaction.t_type() {
            TransactionType::Deposit => {
                if let Some(amount) = transaction.amount() {
                    self.transactions
                        .insert(transaction.id(), (transaction.client_id(), amount));
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = transaction.amount() {
                    self.transactions
                        .insert(transaction.id(), (transaction.client_id(), -amount));
                    // withdrawal is recorded as negative amount
                }
            }
            _ => {}
        }
    }

    // pop a transaction from the record
    pub fn pop_transaction(
        &mut self,
        transaction_id: &TransactionID,
    ) -> Option<(TransactionID, (ClientID, Amount))> {
        self.transactions.remove_entry(transaction_id)
    }

    // recover a disputed transaction
    pub fn recover_transaction(
        &mut self,
        transaction_id: TransactionID,
        client_id: ClientID,
        amount: Amount,
    ) {
        self.transactions
            .insert(transaction_id, (client_id, amount));
    }
}
