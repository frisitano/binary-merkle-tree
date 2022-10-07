use crate::{indices, DBValue, HashDBRef, Hasher, Tree, TreeError, TreeRecorder, EMPTY_PREFIX, Node};

pub struct TreeDBBuilder<'db, H: Hasher> {
    db: &'db dyn HashDBRef<H, DBValue>,
    root: &'db H::Out,
    depth: usize,
    recorder: Option<&'db mut dyn TreeRecorder>,
}

impl<'db, H: Hasher> TreeDBBuilder<'db, H> {
    pub fn new(db: &'db dyn HashDBRef<H, DBValue>, root: &'db H::Out, depth: usize) -> Self {
        Self {
            db,
            root,
            depth,
            recorder: None,
        }
    }

    pub fn with_recorder<'recorder: 'db>(
        mut self,
        recorder: &'recorder mut dyn TreeRecorder,
    ) -> Self {
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

    pub fn build(self) -> TreeDB<'db, H> {
        TreeDB {
            db: self.db,
            root: self.root,
            depth: self.depth,
            recorder: self.recorder.map(core::cell::RefCell::new),
        }
    }
}

/// A `Tree` implementation using a generic `HashDBRef` backing database and a generic `Hasher`
/// to generate keys.
///
/// Use it as a `Tree` trait object.  You can use `db()` (`db_mut()`) to get the (mutable) backing
/// `HashDBRef` database object.
pub struct TreeDB<'a, H: Hasher> {
    db: &'a dyn HashDBRef<H, DBValue>,
    root: &'a H::Out,
    depth: usize,
    recorder: Option<core::cell::RefCell<&'a mut dyn TreeRecorder>>,
}

impl<'a, H: Hasher> TreeDB<'a, H> {
    /// Get the backing database.
    pub fn db(&self) -> &dyn HashDBRef<H, DBValue> {
        self.db
    }

    pub fn get(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        // if index < 1 || (1 << self.depth) * 3 <= index {
        //     return Err(TreeError::IndexOutOfBounds);
        // }
        let root_data = self.db.get(self.root).ok_or(TreeError::DataNotFound)?;
        let mut current_node: Node;
        current_node = bincode::deserialize(&root_data).unwrap();

        for &bit in key {
            if let Node::Inner(left, right) = current_node {
                let key = if bit == 0 { left } else { right };
                let data = self.db.get(&left).ok_or(TreeError::DataNotFound)?;
                current_node = bincode::deserialize(&data).unwrap();
            } else {
                return Err(TreeError::UnexpectedNodeType)
            }
        }

        match current_node {
            Node::Leaf(value) => Ok(value),
            Node::Value(value) => Ok(value),
            Node::Inner(_,_) => Err(TreeError::UnexpectedNodeType)
        }
    }
}

// impl<'a, H: Hasher> Tree<H> for TreeDB<'a, H> {
//     fn root(&self) -> &H::Out {
//         self.root
//     }
//
//     fn depth(&self) -> usize {
//         self.depth
//     }
//
//     fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError> {
//         // if (1 << self.depth) <= offset {
//         //     return Err(TreeError::IndexOutOfBounds);
//         // }
//
//         // let value_index = indices::value_index(offset, self.depth);
//         let result = self.get(value_index);
//
//         self.recorder
//             .as_ref()
//             .map(|r| r.borrow_mut().record(value_index));
//
//         result
//     }
//
//     fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError> {
//         if (1 << self.depth) <= offset {
//             return Err(TreeError::IndexOutOfBounds);
//         }
//
//
//
//         let leaf_index = indices::leaf_index(offset, self.depth);
//         let result = self.get(leaf_index);
//
//         self.recorder
//             .as_ref()
//             .map(|r| r.borrow_mut().record(leaf_index));
//
//         result
//     }
//
//     fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError> {
//         // if (1 << self.depth) <= offset {
//         //     return Err(TreeError::IndexOutOfBounds);
//         // }
//
//         // let leaf_index = indices::leaf_index(offset, self.depth);
//         let mut proof = Vec::new();
//
//         let mut authentication_nodes =
//             indices::authentication_indices(&[leaf_index], true, self.depth);
//         authentication_nodes.push(leaf_index);
//
//         for node_index in authentication_nodes.iter() {
//             let node = self.get(*node_index)?;
//             proof.push((*node_index, node));
//         }
//
//         self.recorder
//             .as_ref()
//             .map(|r| r.borrow_mut().record(leaf_index));
//
//         Ok(proof)
//     }
// }
