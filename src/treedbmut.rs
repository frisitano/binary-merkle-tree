use hash_db::{EMPTY_PREFIX, HashDB, HashDBRef, Hasher};
use crate::{DBValue, indices, TreeError, TreeMut, TreeRecorder, rstd::BTreeMap};

/// Stored item representation.
pub enum Stored<H: Hasher> {
    /// Node hash.
    Hash(H::Out),
    /// Value.
    Value(DBValue)
}

pub struct TreeDBMutBuilder<'db, H: Hasher> {
    db: &'db mut dyn HashDB<H, DBValue>,
    root: &'db mut H::Out,
    depth: usize,
    recorder: Option<&'db mut dyn TreeRecorder>
}

impl<'db, H: Hasher> TreeDBMutBuilder<'db, H> {
    pub fn new(db: &'db mut dyn HashDB<H, DBValue>, root: &'db mut H::Out, depth: usize) -> Self {
        Self {
            db,
            root,
            depth,
            recorder: None
        }
    }

    pub fn with_recorder(mut self, recorder: &'db mut dyn TreeRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_optional_recorder<'recorder: 'db>(mut self, recorder: Option<&'recorder mut dyn TreeRecorder>) -> Self {
        self.recorder = recorder.map(|r| r as _);
        self
    }

    pub fn build(self) -> TreeDBMut<'db, H> {
        TreeDBMut {
            db: self.db,
            storage: BTreeMap::new(),
            uncommitted: Vec::new(),
            root: self.root,
            depth: self.depth,
            recorder: self.recorder.map(core::cell::RefCell::new)
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
    storage: BTreeMap<usize, Stored<H>>,
    uncommitted: Vec<usize>,
    root: &'a mut H::Out,
    depth: usize,
    recorder: Option<core::cell::RefCell<&'a mut dyn TreeRecorder>>
}

impl<'a, H: Hasher> TreeDBMut<'a, H> {
    pub fn new(db: &'a mut dyn HashDB<H, DBValue>, root: &'a mut H::Out, depth: usize) -> Self {
        Self {
            db,
            storage: BTreeMap::new(),
            uncommitted: Vec::new(),
            root,
            depth,
            recorder: None
        }
    }

    pub fn db(&self) -> &dyn HashDB<H, DBValue> {
        self.db
    }

    pub fn db_mut(&mut self) -> &mut dyn HashDB<H, DBValue> {
        self.db
    }

    pub fn get(&self, index: usize) -> Result<DBValue, TreeError> {
        if index < 1 || ((2usize.pow((self.depth + 1) as u32) + 2usize.pow(self.depth as u32)) < index) {
            return Err(TreeError::IndexOutOfBounds)
        };

        match self.storage.get(&index) {
            Some(Stored::Value(value)) => return Ok(value.clone()),
            Some(Stored::Hash(hash)) => return Ok(hash.as_ref().to_vec()),
            None => ()
        }

        let db_key = H::hash(&index.to_le_bytes());
        self.db.get(&db_key, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)
    }

    pub fn commit(&mut self) {
        for node in self.uncommitted.drain(..) {
            let value = &self.storage[&node];
            let key = H::hash(&node.to_le_bytes());

            if self.db.contains(&key, EMPTY_PREFIX) {
                self.db.remove(&key, EMPTY_PREFIX);
            }

            let data = match value {
                Stored::Value(value) => value.clone(),
                Stored::Hash(hash) => hash.as_ref().to_vec()
            };
            self.db.emplace(key, EMPTY_PREFIX, data);

            if node == 1 {
                if let Stored::Hash(root) = value {
                    *self.root = root.clone();
                }
            }
        }
    }
}

impl<'a, H: Hasher> TreeMut<H> for TreeDBMut<'a, H> {
    fn root(&mut self) -> &H::Out {
        self.commit();
        self.root
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn get_value(&self, offset: usize) -> Result<DBValue, TreeError> {
        if 2usize.pow(self.depth as u32) < offset {
            return Err(TreeError::IndexOutOfBounds)
        }
        let value_index = indices::storage_value_index(offset, self.depth);
        let result = self.get(value_index);

        self.recorder.as_ref().map(|recorder| recorder.borrow_mut().record(value_index));

        result
    }

    fn get_leaf(&self, offset: usize) -> Result<DBValue, TreeError> {
        if 2usize.pow(self.depth as u32) < offset {
            return Err(TreeError::IndexOutOfBounds)
        }
        let leaf_index = indices::storage_leaf_index(offset, self.depth);
        let result = self.get(leaf_index);

        self.recorder.as_ref().map(|recorder| recorder.borrow_mut().record(leaf_index));

        result
    }

    fn get_proof(&self, offset: usize) -> Result<Vec<(usize, DBValue)>, TreeError> {
        let leaf_index = indices::storage_leaf_index(offset, self.depth);
        let mut proof = Vec::new();

        let mut authentication_nodes = indices::authentication_indices(&[leaf_index], self.depth);
        authentication_nodes.push(leaf_index);

        for node_index in authentication_nodes.iter() {
            let node = self.get(*node_index)?;
            proof.push((*node_index, node));
        }

        self.recorder.as_ref().map(|recorder| recorder.borrow_mut().record(leaf_index));

        Ok(proof)
    }

    fn insert_value(&mut self, offset: usize, value: DBValue) -> Result<DBValue, TreeError> {
        let old_value = self.get_value(offset)?;
        let leaf = H::hash(&value);
        let value_index = indices::storage_value_index(offset, self.depth);
        let leaf_index = indices::storage_leaf_index(offset, self.depth);
        self.storage.insert(value_index, Stored::Value(value));
        self.storage.insert(leaf_index, Stored::Hash(leaf));
        self.uncommitted.extend([value_index, leaf_index]);

        let mut current_index = leaf_index;
        let mut current_value = leaf;

        for _ in 0..self.depth {
            let sibling_index = indices::sibling_indices(&[current_index])[0];
            let sibling = self.get(sibling_index)?;
            let parent_index = indices::parent_index(current_index);

            let concat_nodes =  if current_index % 2 == 0 {
                let mut concat = current_value.as_mut().to_vec();
                concat.append( &mut sibling.clone());
                concat
            } else {
                let mut concat = sibling.clone().to_vec();
                concat.append( &mut current_value.as_ref().to_vec());
                concat
            };

            let parent_hash = H::hash(&concat_nodes);

            self.storage.insert(parent_index, Stored::Hash(parent_hash));
            self.uncommitted.push(parent_index);

            current_index = parent_index;
            current_value = parent_hash;
        }

        self.recorder.as_ref().map(|recorder| recorder.borrow_mut().record(value_index));

        Ok(old_value)
    }
}
