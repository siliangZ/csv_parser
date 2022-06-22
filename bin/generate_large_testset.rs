use csv_parser::Transaction;
use serde::Serialize;
use std::{fs::File, path::Path};

fn create_big_data_with_different_clients() {
    let mut writer =
        csv::Writer::from_path(Path::new("./large_dataset_with_different_clients.csv")).unwrap();
    let total_clients = 10_000;
    let mut current_tx = 0;
    loop {
        for client_id in 0..total_clients {
            let t = Transaction::new(
                csv_parser::TransactionType::Deposit,
                client_id,
                current_tx,
                Some(2.0),
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Dispute,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Resolve,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();

            current_tx += 1;
            let t = Transaction::new(
                csv_parser::TransactionType::Withdrawal,
                client_id,
                current_tx,
                Some(1.5),
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Dispute,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Resolve,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();
            current_tx += 1;

            let t = Transaction::new(
                csv_parser::TransactionType::Deposit,
                client_id,
                current_tx,
                Some(2.0),
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Dispute,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();
            let t = Transaction::new(
                csv_parser::TransactionType::Chargeback,
                client_id,
                current_tx,
                None,
            );
            writer.serialize(t).unwrap();

            writer.flush().unwrap();
        }
        break;
    }
}

fn main() {
    create_big_data_with_different_clients();
}
