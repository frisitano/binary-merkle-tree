#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod rstd {
    pub use std::{
        collections::{BTreeSet, HashMap},
        convert, mem,
        vec::Vec,
    };
}

#[cfg(not(feature = "std"))]
mod rstd {
    pub use alloc::collections::Vec;
    pub use core::collections::{BTreeSet, HashMap};
    pub use core::mem;
}

mod key;
mod indices;
mod node;
mod proof;
mod recorder;
mod treedb;
mod treedbmut;

#[cfg(test)]
mod test;

use core::fmt::Debug;
use hash_db::{HashDBRef, Hasher, EMPTY_PREFIX};
use std::clone::Clone;

// pub use proof::generate_proof;
pub use node::{decode_hash, Node, NodeHash, Value};
pub use proof::StorageProof;
pub use recorder::Recorder;
pub use treedb::{TreeDB, TreeDBBuilder};
pub use treedbmut::{TreeDBMut, TreeDBMutBuilder};
pub use key::{Key, KeyIter};

/// Database value
pub type DBValue = Vec<u8>;

/// Tree Errors
#[derive(Clone, Debug)]
pub enum TreeError {
    DataNotFound,
    IndexOutOfBounds,
    UnexpectedNodeType,
    NodeDeserializationFailed,
    NodeIndexOutOfBounds,
    DecodeHashFailed,
}

/// An index-value datastore implemented as a database-backed binary merkle tree
/// The tree root, internal nodes and leaves are all of type Hasher::Out.  The
/// values are of type DBValue which is a bytevec.  Tree nodes and values are
/// indexed using the following standard - index = 2^(layer) + offset, where
/// layer is the layer of merkle tree starting from 0 for the root layer and
/// offset is the number of nodes from the left most node in the tree starting
/// from 0.
/// ```text
///       1 *         <- tree root
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
    fn get_leaf(&self, key: &[u8]) -> Result<H::Out, TreeError>;

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
    fn get_leaf(&self, key: &[u8]) -> Result<H::Out, TreeError>;

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError>;

    /// Insert a value at the specified index.  Returns the old value at the specified index.
    fn insert(&mut self, key: &[u8], value: DBValue) -> Result<DBValue, TreeError>;
}

/// A tree recorder that can be used to record tree accesses.
///
/// The `TreeRecorder is used to construct a proof that attests to the inclusion of accessed
/// nodes in a tree.
pub trait TreeRecorder<H: Hasher> {
    /// Record access of the the given node index.
    fn record(&mut self, node: Node<H>);
}
