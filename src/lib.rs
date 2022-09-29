#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod rstd {
    pub use std::{collections::BTreeMap, vec::Vec, mem};
}

#[cfg(not(feature = "std"))]
mod rstd {
    pub use alloc::collections::{BTreeMap, Vec};
    pub use core::mem;
}

mod indices;
mod treedb;
mod treedbmut;
#[cfg(test)]
mod test;

use hash_db::{EMPTY_PREFIX, HashDB, Hasher};

pub use treedb::TreeDB;
pub use treedbmut::TreeDBMut;

/// Database value
pub type DBValue = Vec<u8>;

/// Tree Errors
#[derive(Clone, Debug)]
pub enum TreeError {
    DataNotFound,
    IndexOutOfBounds,
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
/// ```
pub trait Tree<H: Hasher> {
    /// Return the root of the tree.
    fn root(&self) -> &H::Out;

    /// Return the depth of the tree.
    fn depth (&self) -> usize;

    /// Get the tree node hash at the specified index.
    fn get(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get the value at the specified index.
    fn get_value(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get the leaf at the specified index.
    fn get_leaf(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(&self, index: usize) -> Result<Vec<(usize, DBValue)>, TreeError>;
}

/// An index-value datastore implemented as a database-backed binary merkle tree
pub trait TreeMut<H: Hasher> {
    /// Return the root of the tree.
    fn root(&mut self) -> &H::Out;

    /// Return the depth of the tree.
    fn depth(&self) -> usize;

    /// Get the tree node hash at the specified index.
    fn get(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get the value at the specified index.
    fn get_value(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get the leaf hash at the specified index.
    fn get_leaf(&self, index: usize) -> Result<DBValue, TreeError>;

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(&self, index: usize) -> Result<Vec<(usize, DBValue)>, TreeError>;

    /// Insert a value at the specified index.  Returns the old value at the specified index.
    fn insert_value(&mut self, index: usize, value: DBValue) -> Result<DBValue, TreeError>;
}

/// A tree recorder that can be used to record tree accesses.
///
/// The `TreeRecorder is used to construct a proof that attests to the inclusion of accessed
/// nodes in a tree.
pub trait TreeRecorder {
    /// Record access of the the given node index.
    fn record(&mut self, node: usize);
}

/// Record node accesses.
pub struct Recorder {
    nodes: Vec<usize>
}


impl Recorder {
    /// Create a new `Recorder`.
    pub fn new() -> Self { Self { nodes: rstd::Vec::new() } }

    /// Drain all visited nodes.
    pub fn drain(&mut self) -> Vec<usize> {
        rstd::mem::take(&mut self.nodes)
    }
}

impl TreeRecorder for Recorder {
    fn record(&mut self, node: usize) {
        self.nodes.push(node);
    }
}