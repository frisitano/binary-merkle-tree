use crate::{
    decode_hash, indices, DBValue, HashDBRef, Hasher, EncodedNode, Tree, TreeError, TreeRecorder,
    EMPTY_PREFIX,
};

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

    pub fn lookup(&self, key: &H::Out) -> Result<EncodedNode, TreeError> {
        let data = self
            .db
            .get(key, EMPTY_PREFIX)
            .ok_or(TreeError::DataNotFound)?;
        let data = bincode::deserialize(&data).unwrap();
        Ok(data)
    }

    pub fn get(&self, key: &[u8]) -> Result<EncodedNode, TreeError> {
        let mut current_node = self.lookup(self.root)?;

        for &bit in key {
            let key = current_node.get_inner_node_hash::<H>(bit)?;
            current_node = self.lookup(&key)?;
        }

        Ok(current_node)
    }
}

impl<'a, H: Hasher> Tree<H> for TreeDB<'a, H> {
    fn root(&self) -> &H::Out {
        self.root
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        // let value_index = indices::value_index(offset, self.depth);
        let data = self.get(key);

        self.recorder.as_ref().map(|r| r.borrow_mut().record(key));

        data.map(|node| node.get_value())?
    }

    fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self.get(&key[..key.len() - 1]);

        self.recorder.as_ref().map(|r| r.borrow_mut().record(key));

        data.map(|node| node.get_inner_node_value(key[key.len() - 1]))?
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
}
