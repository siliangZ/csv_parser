
use csv_parser::{
    build_csv_reader, ClientAccount, ClientID, Transaction,
    TransactionHashmapDB,
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, io::Read, path::Path, rc::Rc, time::Instant};

/// process the csv that could be loaded into memory through reader
/// reader could point to a file stream or tcp stream
fn process_csv_from_reader<R: Read>(
    stream_reader: R,
    db: &mut ClientDatabase,
    transaction_db: Rc<RefCell<TransactionHashmapDB>>,
) {
    let mut reader = build_csv_reader(stream_reader);
    let mut raw_record = csv::ByteRecord::new();
    let headers = reader.byte_headers().unwrap().clone();
    while reader.read_byte_record(&mut raw_record).unwrap() {
        let transaction: Transaction = raw_record.deserialize(Some(&headers)).unwrap();
        if let Some(client_account) = db.get_mut(&transaction.client_id()) {
            if let Err(e) = client_account.process_transaction(&transaction) {
                println!("{}", e);
            }
        } else {
            let mut client =
                ClientAccount::new_with_db(transaction.client_id(), transaction_db.clone());
            // TODO: remove the unwrap
            if let Err(e) = client.process_transaction(&transaction) {
                println!("{}", e);
            }
            db.insert(transaction.client_id(), client);
        }
    }
}

//#[allow(unused)]
//async fn process_csv_from_reader_async<R: Read>(reader: R, db: &mut ClientDatabase) {
//process_csv_from_reader(reader, db);
//}

fn print_database(db: &mut ClientDatabase) {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_writer(std::io::stdout());
    let records = db.iter().map(|(_, client_account)| &client_account.info);
    for record in records {
        // [TODO] fix the unwrap
        writer.serialize(record).unwrap();
    }
    // [TODO] fix the unwrap
    writer.flush().unwrap();
}

type ClientDatabase = HashMap<ClientID, ClientAccount>;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() > 1);
    println!("parse {:?}", args[1]);
    let path = Path::new(&args[1]);
    let f = std::fs::File::open(path).expect(&format!("can't find input file {:?}", path));
    let transaction_db = Rc::new(RefCell::new(TransactionHashmapDB::new()));
    let mut db = ClientDatabase::new();

    //let now = Instant::now();
    //let handle = tokio::spawn(async move { process_csv_from_reader(f, &mut db) });
    //handle.await.unwrap();
    let now = Instant::now();
    process_csv_from_reader(f, &mut db, transaction_db);
    println!(
        "spent {} ms on processing the dataset",
        now.elapsed().as_millis()
    );
    return;
    print_database(&mut db);
}
