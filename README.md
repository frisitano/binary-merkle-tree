# Binary Merkle Tree

An implementation of a binary merkle tree backed by a `HashDB` database backend.  Supports
a generic `Hasher`.  The implementation is compatible with `no_std` targets by disabling the `std` 
feature (default). 

## Overview
An index-value datastore implemented as a database-backed binary merkle tree. The tree root, internal nodes 
and leaves are all of type `Hasher::Out`. The values are of type `Vec<u8>` (`DBvalue`).  Tree nodes and values 
are indexed using the following standard:
```
index = 2^(layer) + offset
```
where `layer` is the layer of merkle tree starting from 0 for the root layer and
`offset` is the number of nodes from the left most node in the layer starting
from 0.
```text
       1 *        <- tree root
       /   \
      /     \
   2 *      3 *    <- internal nodes
    / \     / \
 4 o   o   o   o   <- leaves
   |   |   |   |
   #   #   #   #   <- values
   8   9   10  11

   0   1   2   3   <- offset
 ```

## Interface
The public traits `Tree` and `TreeMut` implemented by `TreeDB` and `TreeDBMut` respectively, require interaction with 
leaves and values in the tree via specification of their offset in the following methods:

```rust
    /// Get the value at the specified index.
    fn get_value(offset: usize) -> ...

    /// Get the leaf hash at the specified index.
    fn get_leaf(offset: usize) -> ...

    /// Get an inclusion proof for the leaf at the specified index.
    fn get_proof(offset: usize) -> ...

    /// Insert a value at the specified index.  Returns the old value at the specified index.
    fn insert_value(offset: usize, value: DBValue) -> ...
```
