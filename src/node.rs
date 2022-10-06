use crate::{Hasher, DBValue};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Node {
    Value(DBValue),
    Leaf(Vec<u8>),
    Inner(Vec<u8>, Vec<u8>),
}


