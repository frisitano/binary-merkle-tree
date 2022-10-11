use std::{collections::HashMap, thread::current};

use crate::{
    decode_hash, indices, rstd::BTreeMap, DBValue, EncodedNode, TreeError, TreeMut, TreeRecorder,
};
use hash_db::{HashDB, HashDBRef, Hasher, EMPTY_PREFIX};

/// Stored item representation.
pub enum Stored {
    /// Node hash.
    New(EncodedNode),
    /// Value.
    Cached(EncodedNode),
}

impl Stored {
    pub fn get_node(&self) -> &EncodedNode {
        match self {
            Stored::New(node) => node,
            Stored::Cached(node) => node,
        }
    }
}

pub struct TreeDBMutBuilder<'db, H: Hasher> {
    db: &'db mut dyn HashDB<H, DBValue>,
    root: &'db mut H::Out,
    depth: usize,
    recorder: Option<&'db mut dyn TreeRecorder>,
}

impl<'db, H: Hasher> TreeDBMutBuilder<'db, H> {
    pub fn new(db: &'db mut dyn HashDB<H, DBValue>, root: &'db mut H::Out, depth: usize) -> Self {
        Self {
            db,
            root,
            depth,
            recorder: None,
        }
    }

    pub fn with_recorder(mut self, recorder: &'db mut dyn TreeRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_optional_recorder<'recorder: 'db>(
        mut self,
        recorder: Option<&'recorder mut dyn TreeRecorder>,
    ) -> Self {
        self.recorder = recorder.map(|r| r as _);
        self
    }

    pub fn build(self) -> TreeDBMut<'db, H> {
        TreeDBMut {
            db: self.db,
            storage: HashMap::new(),
            root: self.root,
            depth: self.depth,
            recorder: self.recorder.map(core::cell::RefCell::new),
        }
    }
}

/// A `TreeMut` implementation using a generic `HashDB` backing database.
///
/// Use it as a `TreeMut` trait object.  You can use `db()` to get the backing
/// database object.  Changes are not committed until `commit()` is called.
///
/// Querying the root or dropping the `TreeDBMut` will `commit()` stored changes.
pub struct TreeDBMut<'a, H: Hasher> {
    db: &'a mut dyn HashDB<H, DBValue>,
    storage: HashMap<H::Out, Stored>,
    root: &'a mut H::Out,
    depth: usize,
    recorder: Option<core::cell::RefCell<&'a mut dyn TreeRecorder>>,
}

impl<'a, H: Hasher> TreeDBMut<'a, H> {
    pub fn new(db: &'a mut dyn HashDB<H, DBValue>, root: &'a mut H::Out, depth: usize) -> Self {
        Self {
            db,
            storage: HashMap::new(),
            root,
            depth,
            recorder: None,
        }
    }

    pub fn db(&self) -> &dyn HashDB<H, DBValue> {
        self.db
    }

    pub fn db_mut(&mut self) -> &mut dyn HashDB<H, DBValue> {
        self.db
    }

    pub fn lookup(&self, key: &H::Out) -> Result<EncodedNode, TreeError> {
        if let Some(value) = self.storage.get(key) {
            return Ok(value.get_node().clone());
        }

        let data = self
            .db
            .get(key, EMPTY_PREFIX)
            .ok_or(TreeError::DataNotFound)?;
        let node: EncodedNode = bincode::deserialize(&data).unwrap();

        Ok(node)
    }

    pub fn get(&self, key: &[u8]) -> Result<EncodedNode, TreeError> {
        // if index < 1 || (1 << self.depth) * 3 <= index {
        //     return Err(TreeError::IndexOutOfBounds);
        // }
        let mut current_node = self.lookup(self.root)?;

        for &bit in key {
            let key = current_node.get_inner_node_hash::<H>(bit)?;
            current_node = self.lookup(&key)?;
        }

        Ok(current_node)
    }

    fn insert_at(
        &mut self,
        current_node: &mut EncodedNode,
        key: &[u8],
        value: DBValue,
    ) -> Result<EncodedNode, TreeError> {
        if key.len() == 1 {
            let old_leaf = current_node.get_inner_node_hash::<H>(key[0])?;
            let old_value = self.lookup(&old_leaf)?;
            let new_node = EncodedNode::Value(value);
            let new_leaf = new_node.hash::<H>();
            current_node.set_inner_node_data(key[0], new_leaf.as_ref().to_vec())?;
            self.storage.insert(new_leaf, Stored::New(new_node));
            Ok(old_value)
        } else {
            let child_key = current_node.get_inner_node_hash::<H>(key[0])?;
            // TODO should lookup storage first
            let mut child_node = self.lookup(&child_key)?;
            let old_value = self.insert_at(&mut child_node, &key[1..], value)?;
            let child_hash = child_node.hash::<H>();
            current_node.set_inner_node_data(key[0], child_hash.as_ref().to_vec())?;
            self.storage.insert(child_hash, Stored::New(child_node));
            Ok(old_value)
        }
    }

    // pub fn commit(&mut self) {
    //     for node in self.uncommitted.drain(..) {
    //         let value = &self.storage[&node];
    //         let key = H::hash(&node.to_le_bytes());

    //         if self.db.contains(&key, EMPTY_PREFIX) {
    //             self.db.remove(&key, EMPTY_PREFIX);
    //         }

    //         let data = match value {
    //             Stored::Value(value) => value.clone(),
    //             Stored::Hash(hash) => hash.as_ref().to_vec(),
    //         };
    //         self.db.emplace(key, EMPTY_PREFIX, data);

    //         if node == 1 {
    //             if let Stored::Hash(root) = value {
    //                 *self.root = root.clone();
    //             }
    //         }
    //     }
    // }
}

impl<'a, H: Hasher> TreeMut<H> for TreeDBMut<'a, H> {
    fn root(&mut self) -> &H::Out {
        // self.commit();
        self.root
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self.get(key).map(|node| node.get_value())?;
        self.recorder.as_ref().map(|r| r.borrow_mut().record(key));

        data
    }

    fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self
            .get(&key[..key.len() - 1])
            .map(|node| node.get_inner_node_value(key[key.len() - 1]))?;
        self.recorder.as_ref().map(|r| r.borrow_mut().record(key));

        data
    }

    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let mut proof = Vec::new();
        proof.push((1, self.root.as_ref().to_vec()));

        let root_data = self
            .db
            .get(self.root, EMPTY_PREFIX)
            .ok_or(TreeError::DataNotFound)?;

        let mut current_node: EncodedNode;
        current_node = bincode::deserialize(&root_data).unwrap();

        for (i, &bit) in key.iter().enumerate() {
            let index = indices::compute_index(&key[..i + 1]);
            let left_index = if index % 2 == 0 { index } else { index ^ 1 };

            if let EncodedNode::Inner(left, right) = current_node {
                let hash_left = decode_hash::<H>(&left).unwrap();
                let hash_right = decode_hash::<H>(&right).unwrap();
                let key = if bit == 0 { hash_left } else { hash_right };
                let data = self
                    .db
                    .get(&key, EMPTY_PREFIX)
                    .ok_or(TreeError::DataNotFound)?;
                current_node = bincode::deserialize(&data).unwrap();

                proof.extend_from_slice(&[
                    (left_index, hash_left.as_ref().to_vec()),
                    (left_index + 1, hash_right.as_ref().to_vec()),
                ]);
            } else {
                return Err(TreeError::UnexpectedNodeType);
            }
        }

        proof.push((0, current_node.get_value()?));

        self.recorder.as_ref().map(|r| r.borrow_mut().record(key));

        Ok(proof)
    }

    fn insert_value(&mut self, key: &[u8], value: DBValue) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        };

        let mut root_data: EncodedNode = self.lookup(self.root)?;

        let old_value = self.insert_at(&mut root_data, key, value)?;

        *self.root = root_data.hash::<H>();
        self.storage
            .insert(self.root.to_owned(), Stored::New(root_data));

        self.recorder
            .as_ref()
            .map(|recorder| recorder.borrow_mut().record(key));

        old_value.get_value()
    }
}
