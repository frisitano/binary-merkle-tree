use crate::{DBValue, indices, Tree, TreeError, EMPTY_PREFIX, HashDB, Hasher, TreeRecorder};

pub struct TreeDBBuilder<'db, H: Hasher> {
    db: &'db mut dyn HashDB<H, DBValue>,
    root: &'db H::Out,
    depth: usize,
    recorder: Option<&'db mut dyn TreeRecorder>,
}

impl<'db, H: Hasher> TreeDBBuilder<'db, H> {
    pub fn new (db: &'db mut dyn HashDB<H, DBValue>, root: &'db H::Out, depth: usize) -> Self {
        Self {
            db,
            root,
            depth,
            recorder: None
        }
    }

    pub fn with_recorder<'recorder: 'db>(mut self, recorder: &'recorder mut dyn TreeRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_optional_recorder<'recorder: 'db>(mut self, recorder: Option<&'recorder mut dyn TreeRecorder>) -> Self {
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

/// A `Tree` implementation using a generic `HashDB` backing database and a generic `Hasher`
/// to generate keys.
///
/// Use it as a `Tree` trait object.  You can use `db()` (`db_mut()`) to get the (mutable) backing
/// `HashDB` database object.
pub struct TreeDB<'a, H: Hasher> {
    db: &'a mut dyn HashDB<H, DBValue>,
    root: &'a H::Out,
    depth: usize,
    recorder: Option<core::cell::RefCell<&'a mut dyn TreeRecorder>>,
}


impl<'a, H: Hasher> TreeDB<'a, H> {
    /// Get the backing database.
    pub fn db(&self) -> &dyn  HashDB<H, DBValue> {
        self.db
    }

    /// Get a mutable reference to the backing database.
    pub fn db_mut(&mut self) -> &mut dyn HashDB<H, DBValue> {
        self.db
    }
}

impl<'a, H: Hasher> Tree<H> for TreeDB<'a, H> {
    fn root(&self) -> &H::Out {
        self.root
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn get(&self, index: usize) -> Result<DBValue, TreeError> {
        if index < 1 || (2usize.pow((self.depth + 1) as u32) + 2usize.pow(self.depth as u32)) < index {
            return Err(TreeError::IndexOutOfBounds)
        }

        let key = H::hash(&index.to_le_bytes());
        self.recorder.as_ref().map(|r| r.borrow_mut().record(index));
        self.db.get(&key, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)
    }

    fn get_value(&self, index: usize) -> Result<DBValue, TreeError> {
        if 2usize.pow(self.depth as u32) < index {
            return Err(TreeError::IndexOutOfBounds)
        }
        let value_index = indices::storage_value_index(index, self.depth);
        self.get(value_index)
    }

    fn get_leaf(&self, index: usize) -> Result<DBValue, TreeError> {
        if 2usize.pow(self.depth as u32) < index {
            return Err(TreeError::IndexOutOfBounds)
        }
        let leaf_index = indices::storage_leaf_index(index, self.depth);
        self.get(leaf_index)
    }

    fn get_proof(&self, index: usize) -> Result<Vec<(usize, DBValue)>, TreeError> {
        let leaf_index = indices::storage_leaf_index(index, self.depth);
        let mut proof = Vec::new();

        let mut authentication_nodes = indices::authentication_indices(&[leaf_index], self.depth);
        authentication_nodes.push(leaf_index);

        for node_index in authentication_nodes.iter() {
            let node = self.get(*node_index)?;
            proof.push((*node_index, node));
        }

        Ok(proof)
    }
}
