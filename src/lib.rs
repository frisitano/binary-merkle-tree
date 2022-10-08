#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod rstd {
    pub use std::{
        collections::{BTreeMap, BTreeSet},
        mem,
        vec::Vec,
    };
}

#[cfg(not(feature = "std"))]
mod rstd {
    pub use alloc::collections::{BTreeMap, BTreeSet, Vec};
    pub use core::mem;
}

mod indices;
// mod proof;
// mod recorder;
mod treedb;
// mod treedbmut;

#[cfg(test)]
mod test;

use hash_db::{EMPTY_PREFIX, HashDBRef, Hasher};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

// pub use proof::generate_proof;
// pub use recorder::Recorder;
pub use treedb::{TreeDB, TreeDBBuilder};
// pub use treedbmut::{TreeDBMut, TreeDBMutBuilder};

/// Database value
pub type DBValue = Vec<u8>;

/// Node Enumb
/// Variants include: Value, Leaf, Inner
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Node {
    Value(DBValue),
    Inner(Vec<u8>, Vec<u8>),
}

impl Node {
    pub fn hash<H: Hasher>(&self) -> H::Out {
        match self {
            Node::Inner(left, right) => {
                let mut combined = Vec::with_capacity(H::LENGTH * 2);
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                H::hash(&combined)
            },
            Node::Value(value) => H::hash(&value),
        }
    }

    pub fn get_inner_node_data(&self, node: u8) -> Result<Vec<u8>, TreeError> {
        match self {
            Node::Value(_) => Err(TreeError::UnexpectedNodeType),
            Node::Inner(left, right) => if node == 0 { Ok(left.clone()) } else { Ok(right.clone()) }
        }
    }
}

pub fn decode_hash<H: Hasher>(data: &[u8]) -> Option<H::Out> {
	if data.len() != H::LENGTH {
		return None
	}
	let mut hash = H::Out::default();
	hash.as_mut().copy_from_slice(data);
	Some(hash)
}

/// Tree Errors
#[derive(Clone, Debug)]
pub enum TreeError {
    DataNotFound,
    IndexOutOfBounds,
    UnexpectedNodeType,
}

/// An index-value datastore implemented as a database-backed binary merkle tree
/// The tree root, internal nodes and leaves are all of type Hasher::Out.  The
/// values are of type DBValue which is a bytevec.  Tree nodes and values are
/// indexed using the following standard - index = 2^(layer) + offset, where
/// layer is the layer of merkle tree starting from 0 for the root layer and
/// offset is the number of nodes from the left most node in the tree starting
/// from 0.
/// ```text
///       1 *        <- tree root
///       /   \
///      /     \
///   2 *      3 *    <- internal nodes
///    / \     / \
/// 4 o   o   o   o   <- leaves
///   |   |   |   |
///   #   #   #   #   <- values
///   8   9   10  11
///
///   0   1   2   3   <- offset
/// ```
pub trait Tree<H: Hasher> {
    /// Return the root of the tree.
    fn root(&self) -> &H::Out;

    /// Return the depth of the tree.
    fn depth(&self) -> usize;

    /// Get the value at the specified index.
    fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError>;

    /// Get the leaf at the specified index.
    fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError>;

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError>;
}

/// An index-value datastore implemented as a database-backed binary merkle tree
pub trait TreeMut<H: Hasher> {
    /// Return the root of the tree.
    fn root(&mut self) -> &H::Out;

    /// Return the depth of the tree.
    fn depth(&self) -> usize;

    /// Get the value at the specified index.
    fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError>;

    /// Get the leaf hash at the specified index.
    fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError>;

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError>;

    /// Insert a value at the specified index.  Returns the old value at the specified index.
    fn insert_value(&mut self, key: &[u8], value: DBValue) -> Result<DBValue, TreeError>;
}

/// A tree recorder that can be used to record tree accesses.
///
/// The `TreeRecorder is used to construct a proof that attests to the inclusion of accessed
/// nodes in a tree.
pub trait TreeRecorder {
    /// Record access of the the given node index.
    fn record(&mut self, key: &[u8]);
}
