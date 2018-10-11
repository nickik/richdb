#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(custom_derive)]
#![feature(integer_atomics)]

extern crate rpds;
extern crate rocket;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde as rmps;
extern crate rocksdb;

use rocksdb::{DB, Writable};

use serde::{Deserialize, Serialize};
use rmps::{Deserializer, Serializer};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Cardinality {
    One,
    Many
}

#[derive(Debug)]
pub enum ValueType {
    I64,
    F64,
    VString
}

type Entity = i64;
type TrxIndex = u64;

#[derive(Debug)]
pub struct AttributeValue {
    name : String,
    cardinality : Cardinality,
    component : bool,
    unique : bool,
    ident : String,
    value_type : ValueType
}

trait Attribute {
    // Static method signature; `Self` refers to the implementor type.
    fn new(name: &'static str, value_type: ValueType) -> Self;

    fn name(&self) -> &'static str;
    fn cardinality(&self) -> Cardinality;
    fn component(&self) -> bool;
    fn unique(&self) -> bool;
    fn ident(&self) -> &'static str;
    fn value_type(&self) -> ValueType;
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub enum Direction {
    Assert,
    Retract
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub enum DatomValue {
    I64(i64),
    F64(f64),
    Entity(Entity),
    String(String)
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct FullStorageDatum {
    entity: Entity,
    attribute: Entity,
    direction: Direction,
    tx: TrxIndex,
    value: DatomValue
}


#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct TransactionLogDatom {
    entity: Entity,
    attribute: Entity,
    direction: Direction,
    value: DatomValue
}

fn trx_index_to_key(trx_index : TrxIndex) ->  &[u8] {

    let mut index_buf = Vec::new();
    trx_index.serialize(&mut Serializer::new(&mut index_buf)).unwrap()
}


struct RichDbError {
    str : &'static str
}

fn to_full(trx_index : TrxIndex, datum : TransactionLogDatom) -> FullStorageDatum {

    FullStorageDatum {
        entity: datum.entity,
        attribute: datum.attribute,
        direction: datum.direction,
        tx: trx_index,
        value: datum.value
    }
}


fn load_from_transaction_load(db : &mut DB, trx_index : TrxIndex) -> Result<FullStorageDatum, RichDbError> {

    match db.get(trx_index_to_key(trx_index)) {
        Ok(Some(value)) => {
            let mut de = Deserializer::new(&value[..]);
            let val2 : TransactionLogDatom = Deserialize::deserialize(&mut de).unwrap();

            Ok(to_full(trx_index, val2))
        },
        Ok(None) => Err(RichDbError { str : "trx_index not found" }),
        Err(e) => {
            println!("operational problem encountered: {}", e);
            Err(RichDbError { str : "operational problem encountered" })
        },
    }
}

fn trx_index_store(db : &mut DB, key : &[u8], trx_index : TrxIndex) {

    let val = db.put(key, trx_index_to_key(trx_index));
}

fn trx_index_load(db : &DB, key : &[u8]) -> TrxIndex {

    match db.get(key) {
        Ok(Some(value)) => {
            let mut de = Deserializer::new(&value[..]);
            let val2 : TrxIndex = Deserialize::deserialize(&mut de).unwrap();
            val2
        },
        Ok(None) => 0,
        Err(e) => {
            println!("operational problem encountered: {}", e);
            0
        },
    }
}

fn trx_index_load_inc(db : &mut DB) -> TrxIndex {

    let k = b"TRX_LOG_INDEX";

    let current = trx_index_load(db, k);

    trx_index_store(db, k, current + 1);

    return current;
}


fn attach_tl(db : &mut DB, datum : &TransactionLogDatom) -> TrxIndex {

    let new_key = trx_index_load_inc(db);
    let mut key_buf = Vec::new();
    new_key.serialize(&mut Serializer::new(&mut key_buf)).unwrap();

    let mut value_buf = Vec::new();
    datum.serialize(&mut Serializer::new(&mut value_buf)).unwrap();

    db.put(&key_buf, &value_buf);

    return new_key;
}

fn main() {

    let val = FullStorageDatum {
        entity: 1,
        attribute: 2,
        direction: Direction::Assert,
        tx: 0,
        value: DatomValue::Entity(1)
    };

    let insert_list = vec![
        TransactionLogDatom {
            entity: 1,
            attribute: 2,
            direction: Direction::Assert,
            value: DatomValue::Entity(1)
        },
        TransactionLogDatom {
            entity: 1,
            attribute: 2,
            direction: Direction::Retract,
            value: DatomValue::Entity(1)
        }
    ];

    let mut db = DB::open_default("/tmp").unwrap();

    let key_vec: Vec<TrxIndex> = insert_list.iter().map(|item| attach_tl(&mut db, item)).collect();



    println!("{:?}", key_vec);


    println!("Datum {}",  load_from_transaction_load(&mut db, key_vec[0]).unwrap() );
    println!("Datum {}",  load_from_transaction_load(&mut db, key_vec[0]).unwrap() );


    //let mut de = Deserializer::new(&buf[..]);
    //let val2 : StorageDatum = Deserialize::deserialize(&mut de).unwrap();
    //println!("{:?}", val2);

    /*db.put(b"my key",&buf);

    let val2:  = match db.get(b"my key") {
        Ok(Some(value)) : value,
        Ok(None) => println!("value not found"),
        Err(e) => println!("operational problem encountered: {}", e),
    }*/
}