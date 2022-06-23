use csv_parser::{build_csv_reader, ClientAccount, ClientID, Transaction, TransactionHashmapDB};
use std::{cell::RefCell, collections::HashMap, io::Read, path::Path, rc::Rc};

/// process the csv that could be loaded into memory through reader
/// reader could point to a file stream or tcp stream
fn process_csv_from_reader<R: Read>(
    stream_reader: R,
    db: &mut ClientDatabase,
    transaction_db: Rc<RefCell<TransactionHashmapDB>>,
) {
    let mut reader = build_csv_reader(stream_reader);
    let mut raw_record = csv::ByteRecord::new();
    let headers = reader
        .byte_headers()
        .expect("the csv should have a header")
        .clone();
    while reader
        .read_byte_record(&mut raw_record)
        .expect("can't read from the csv")
    {
        let transaction: Transaction = raw_record
            .deserialize(Some(&headers))
            .expect("failed to serialize the record to Transaction");
        if let Some(client_account) = db.get_mut(&transaction.client_id()) {
            // ignore the error, could add error handling here when we need to process error case
            let _ = client_account.process_transaction(&transaction);
        } else {
            let mut client =
                ClientAccount::new_with_db(transaction.client_id(), transaction_db.clone());
            // ignore the error, could add error handling here when we need to process error case
            let _ = client.process_transaction(&transaction);
            db.insert(transaction.client_id(), client);
        }
    }
}

fn print_database(db: &mut ClientDatabase) {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_writer(std::io::stdout());
    let records = db.iter().map(|(_, client_account)| &client_account.info);
    for record in records {
        let _ = writer.serialize(record);
    }
    writer.flush().expect("can't flush the buffer of writer");
}

type ClientDatabase = HashMap<ClientID, ClientAccount>;

fn main() {
    // parse out the input file path
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() > 1);
    let path = Path::new(&args[1]);
    let f =
        std::fs::File::open(path).unwrap_or_else(|_| panic!("can't find input file {:?}", path));

    // create transaction database and client database
    let transaction_db = Rc::new(RefCell::new(TransactionHashmapDB::new()));
    let mut db = ClientDatabase::new();

    process_csv_from_reader(f, &mut db, transaction_db);
    print_database(&mut db);
}
