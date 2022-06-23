use serde::{Serialize, Serializer};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    Amount, ClientID, Error, Transaction, TransactionHashmapDB, TransactionID, TransactionType,
};

fn precision_serialize<S>(x: &Amount, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    //let x_str = format!("{:.4}", x);
    //s.serialize_str(&x_str)
    let x = (x * 10000.0).round() / 10000.0;
    s.serialize_f32(x)
}

// the information of client account
#[derive(Serialize, Clone)]
pub struct ClientAccountInfo {
    pub client: ClientID,
    // the total funds that are available for trading, staking, withdrawal, etc
    #[serde(serialize_with = "precision_serialize")]
    pub available: Amount,
    // the total funds that are held for dispute
    #[serde(serialize_with = "precision_serialize")]
    pub held: Amount,
    // the total funds that are available or held
    #[serde(serialize_with = "precision_serialize")]
    pub total: Amount,
    // the accound is locked if a charge back occurs
    #[serde(rename = "locked")]
    pub is_locked: bool,
}

impl ClientAccountInfo {
    pub fn new(client: ClientID) -> Self {
        Self {
            client,
            available: 0f32,
            held: 0f32,
            total: 0f32,
            is_locked: false,
        }
    }
}

/// the account of client. it process all transactions belong to the account
pub struct ClientAccount {
    pub info: ClientAccountInfo,
    // a pointer to a transaction database
    transaction_db: Rc<RefCell<TransactionHashmapDB>>,
    // the transactions that are in dispute
    // record the amount to avoid double checking
    dispute_transactions: HashMap<TransactionID, Amount>,
}

impl ClientAccount {
    pub fn new(clinet_id: ClientID) -> Self {
        let info = ClientAccountInfo::new(clinet_id);
        Self {
            info,
            transaction_db: Rc::new(RefCell::new(TransactionHashmapDB::new())),
            dispute_transactions: HashMap::new(),
        }
    }

    pub fn new_with_db(
        clinet_id: ClientID,
        transaction_db: Rc<RefCell<TransactionHashmapDB>>,
    ) -> Self {
        let info = ClientAccountInfo::new(clinet_id);
        Self {
            info,
            transaction_db,
            dispute_transactions: HashMap::new(),
        }
    }

    // validate the transaction by checking the amount is a valid(we could guarantee that when we parse the data, but it is still good to check it here)
    fn validate_transaction(&self, transaction: &Transaction) -> Result<(), Error> {
        if self.info.is_locked {
            return Err(Error::AccountLocked {
                client: self.info.client,
            });
        }

        if transaction.client_id() != self.info.client {
            return Err(Error::WrongAccount);
        }
        match transaction.t_type() {
            TransactionType::Deposit | TransactionType::Withdrawal => transaction
                .amount()
                .ok_or(Error::InvalidTransaction {
                    client: self.info.client,
                    tx: transaction.id(),
                    t_type: transaction.t_type(),
                    amount: transaction.amount(),
                })
                .and_then(|amount| {
                    if amount < 0f32 {
                        return Err(Error::InvalidTransaction {
                            client: self.info.client,
                            tx: transaction.id(),
                            t_type: transaction.t_type(),
                            amount: transaction.amount(),
                        });
                    }
                    Ok(())
                }),
            _ => Ok(()),
        }
    }

    /// deposit some amount into the account. It is always welcome to deposit :)
    fn deposit(&mut self, transaction: &Transaction) -> Result<(), Error> {
        if let Some(amount) = transaction.amount() {
            self.info.available += amount;
            self.info.total += amount;
            self.transaction_db
                .borrow_mut()
                .add_transaction(transaction);
        }
        Ok(())
    }

    /// withdrawal money from the account
    pub fn withdrawal(&mut self, transaction: &Transaction) -> Result<(), Error> {
        if let Some(amount) = transaction.amount() {
            // sufficient account and sufficient available
            if self.info.total < amount || self.info.available < amount {
                return Err(Error::NoSufficientFunds {
                    client: self.info.client,
                    available: self.info.available,
                    withdrawal: amount,
                });
            } else {
                self.info.available -= amount;
                self.info.total -= amount;
                self.transaction_db
                    .borrow_mut()
                    .add_transaction(transaction);
            }
        }
        Ok(())
    }

    /// dispute a previous transaction. It could be deposit and withdrawal
    pub fn dispute(&mut self, transaction: &Transaction) -> Result<(), Error> {
        // find the previous transaction and pop it outof database
        if let Some((_, (client_id, tx_amount))) = self
            .transaction_db
            .borrow_mut()
            .pop_transaction(&transaction.id())
        {
            // check the current client owns the transactions that will be disputed
            if client_id == self.info.client {
                self.dispute_transactions
                    .insert(transaction.id(), tx_amount);
                self.info.available -= tx_amount;
                self.info.held += tx_amount;
            } else {
                println!("Wrong");
            }
            Ok(())
        } else {
            Err(Error::NotFoundTransaction {
                client: self.info.client,
                tx: transaction.id(),
            })
        }
    }

    /// resolve a previous dispute and recover the disputed transaction back to record
    pub fn resolve(&mut self, transaction: &Transaction) -> Result<(), Error> {
        // find the previous disputed transaction
        if let Some(amount) = self.dispute_transactions.remove(&transaction.id()) {
            self.info.held -= amount;
            self.info.available += amount;
            // if the transaction is resolved, add it back to history for a possible future dispute
            self.transaction_db.borrow_mut().recover_transaction(
                transaction.id(),
                self.info.client,
                amount,
            );
            Ok(())
        } else {
            Err(Error::NotFoundTransaction {
                client: self.info.client,
                tx: transaction.id(),
            })
        }
    }

    /// a solution to dispute and will lock the account
    /// chargeback the dispute on withdrawal is kind of ambiguous
    /// we allow it here which means the client put money back
    pub fn chargeback(&mut self, transaction: &Transaction) -> Result<(), Error> {
        if let Some(amount) = self.dispute_transactions.remove(&transaction.id()) {
            self.info.held -= amount;
            self.info.total -= amount;
            self.info.is_locked = true;
            // the transaction won't get back to history for future dispute
            Ok(())
        } else {
            Err(Error::NotFoundTransaction {
                client: self.info.client,
                tx: transaction.id(),
            })
        }
    }

    pub fn process_transaction(&mut self, transaction: &Transaction) -> Result<(), Error> {
        self.validate_transaction(transaction)?;
        match transaction.t_type() {
            TransactionType::Deposit => self.deposit(transaction),
            TransactionType::Withdrawal => self.withdrawal(transaction),
            TransactionType::Dispute => self.dispute(transaction),
            TransactionType::Resolve => self.resolve(transaction),
            TransactionType::Chargeback => self.chargeback(transaction),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ClientAccount, ClientAccountInfo, Error, Transaction};

    #[test]
    fn test_serialize_client_account_info() {
        let account_info = ClientAccountInfo::new(0);
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b',')
            .from_writer(vec![]);
        writer.serialize(account_info).unwrap();
        let data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!(
            data,
            "client,available,held,total,locked\n0,0.0,0.0,0.0,false\n"
        );
    }

    #[test]
    fn test_precision_in_serialization() {
        let mut account_info = ClientAccountInfo::new(0);
        account_info.available = 0.123;
        account_info.held = 0.1234;
        account_info.total = 0.12345;
        account_info.is_locked = true;
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b',')
            .from_writer(vec![]);
        writer.serialize(account_info).unwrap();
        let data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!(
            data,
            "client,available,held,total,locked\n0,0.123,0.1234,0.1235,true\n"
        );
    }

    #[test]
    fn test_deposit() {
        let mut account = ClientAccount::new(0);
        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Deposit,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, amount);
        assert_eq!(account.info.held, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
    }
    #[test]
    fn test_withdrawal() {
        let mut account = ClientAccount::new(0);
        let current_available = 10f32;
        account.info.available = current_available;
        account.info.total = current_available;

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Withdrawal,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, current_available - amount);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0f32);
    }

    #[test]
    fn test_below_zero_deposit_and_withdrawal() {
        let mut account = ClientAccount::new(0);

        let amount = -2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Deposit,
            account.info.client,
            0,
            Some(amount),
        );
        let result = account.process_transaction(&transaction);
        assert!(matches!(result, Err(Error::InvalidTransaction { .. })))
    }

    #[test]
    fn test_invalid_withdrawal() {
        let mut account = ClientAccount::new(0);

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Withdrawal,
            account.info.client,
            0,
            Some(amount),
        );
        let result = account.process_transaction(&transaction);
        assert!(matches!(
            result,
            Err(crate::Error::NoSufficientFunds { .. })
        ));
        assert_eq!(account.info.available, 0f32);
        assert_eq!(account.info.held, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
    }

    #[test]
    fn test_dispute_deposit() {
        let mut account = ClientAccount::new(0);

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Deposit,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, amount);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0f32);

        let dispute = Transaction::new(
            crate::TransactionType::Dispute,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&dispute).unwrap();
        assert_eq!(account.info.available, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, amount);
        assert!(account.dispute_transactions.len() > 0);
        assert_eq!(account.dispute_transactions.get(&0), Some(&amount));
    }

    #[test]
    fn test_dispute_withdrawal() {
        let mut account = ClientAccount::new(0);
        let current_available = 10f32;
        account.info.available = current_available;
        account.info.total = current_available;

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Withdrawal,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, current_available - amount);
        assert_eq!(account.info.held, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );

        let dispute = Transaction::new(
            crate::TransactionType::Dispute,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&dispute).unwrap();
        assert_eq!(account.info.available, current_available);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, -amount);
        assert!(account.dispute_transactions.len() > 0);
        assert_eq!(account.dispute_transactions.get(&0), Some(&-amount));
    }

    #[test]
    fn test_invalid_dispute() {
        let mut account = ClientAccount::new(0);

        let transaction = Transaction::new(
            crate::TransactionType::Dispute,
            account.info.client,
            0,
            None,
        );

        let result = account.process_transaction(&transaction);
        assert!(matches!(result, Err(Error::NotFoundTransaction { .. })))
    }

    #[test]
    fn test_resolve_dispute() {
        let mut account = ClientAccount::new(0);

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Deposit,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, amount);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0f32);

        let dispute = Transaction::new(
            crate::TransactionType::Dispute,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&dispute).unwrap();
        assert_eq!(account.info.available, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, amount);
        assert!(account.dispute_transactions.len() > 0);
        assert_eq!(account.dispute_transactions.get(&0), Some(&amount));

        let resolve = Transaction::new(
            crate::TransactionType::Resolve,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&resolve).unwrap();
        assert_eq!(account.info.available, amount);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0f32);
        assert!(account.dispute_transactions.is_empty());
        //assert!(account.history_transactions.contains_key(&0)); // recover the transaction back to record
        assert!(account
            .transaction_db
            .borrow_mut()
            .pop_transaction(&0)
            .is_some());
    }

    #[test]
    fn test_chargeback() {
        let mut account = ClientAccount::new(0);

        let amount = 2.3f32;
        let transaction = Transaction::new(
            crate::TransactionType::Deposit,
            account.info.client,
            0,
            Some(amount),
        );
        account.process_transaction(&transaction).unwrap();
        assert_eq!(account.info.available, amount);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0f32);

        let dispute = Transaction::new(
            crate::TransactionType::Dispute,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&dispute).unwrap();
        assert_eq!(account.info.available, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, amount);
        assert!(account.dispute_transactions.len() > 0);
        assert_eq!(account.dispute_transactions.get(&0), Some(&amount));

        let chargeback = Transaction::new(
            crate::TransactionType::Chargeback,
            account.info.client,
            0,
            None,
        );
        account.process_transaction(&chargeback).unwrap();
        assert_eq!(account.info.available, 0f32);
        assert_eq!(
            account.info.total,
            account.info.available + account.info.held
        );
        assert_eq!(account.info.held, 0.0);
        assert!(account.dispute_transactions.is_empty());
    }
}
