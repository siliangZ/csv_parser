use serde::{Deserialize, Serialize};

pub type ClientID = u16;
pub type TransactionID = u32;
pub type Amount = f32;

//#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
//pub struct ClientID(pub u16);
//impl fmt::Display for ClientID {
//fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//// Use `self.number` to refer to each positional data point.
//write!(f, "{}", self.0)
//}
//}

//#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
//pub struct TransactionID(pub u32); // a unique id for transaction
//impl fmt::Display for TransactionID {
//fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//// Use `self.number` to refer to each positional data point.
//write!(f, "{}", self.0)
//}
//}

//#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
//pub struct Amount(pub f32);
//impl fmt::Display for Amount {
//fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//// Use `self.number` to refer to each positional data point.
//write!(f, "{}", self.0)
//}
//}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Transaction {
    #[serde(rename = "type")]
    t_type: TransactionType,
    client: ClientID,
    tx: TransactionID,
    amount: Option<Amount>,
}

impl Transaction {
    pub fn new(
        t_type: TransactionType,
        client: ClientID,
        tx: TransactionID,
        amount: Option<Amount>,
    ) -> Self {
        Self {
            t_type,
            client,
            tx,
            amount,
        }
    }

    pub fn t_type(&self) -> TransactionType {
        self.t_type
    }

    pub fn client_id(&self) -> ClientID {
        self.client
    }

    pub fn id(&self) -> TransactionID {
        self.tx
    }

    pub fn amount(&self) -> Option<Amount> {
        self.amount
    }
}

#[cfg(test)]
mod tests {
    use crate::{build_csv_reader, Transaction};
    use std::fs::File;

    #[test]
    fn serialize_and_deserialize_transaction_from_csv() {
        let f = File::open("./sample_csv/all_transactions.csv").unwrap();
        let mut reader = build_csv_reader(f);
        let mut raw_record = csv::ByteRecord::new();
        let headers = reader.byte_headers().unwrap().clone();
        reader.read_byte_record(&mut raw_record).unwrap();
        let _: Transaction = raw_record.deserialize(Some(&headers)).unwrap();
    }
}
