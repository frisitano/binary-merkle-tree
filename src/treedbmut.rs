use crate::{
    indices, node::NodeHash, node::Value, rstd::HashMap, DBValue, Node, TreeError, TreeMut,
    TreeRecorder, compute_null_hashes,
};
use hash_db::{HashDB, HashDBRef, Hasher, EMPTY_PREFIX};

pub struct TreeDBMutBuilder<'db, H: Hasher> {
    db: &'db mut dyn HashDB<H, DBValue>,
    root: &'db mut H::Out,
    depth: usize,
    recorder: Option<&'db mut dyn TreeRecorder<H>>,
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

    pub fn with_recorder(mut self, recorder: &'db mut dyn TreeRecorder<H>) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_optional_recorder<'recorder: 'db>(
        mut self,
        recorder: Option<&'recorder mut dyn TreeRecorder<H>>,
    ) -> Self {
        self.recorder = recorder.map(|r| r as _);
        self
    }

    pub fn build(self) -> TreeDBMut<'db, H> {
        let root_handle = NodeHash::Hash(*self.root);
        TreeDBMut {
            db: self.db,
            storage: HashMap::new(),
            root: self.root,
            root_handle: root_handle,
            depth: self.depth,
            recorder: self.recorder.map(core::cell::RefCell::new),
            null_hashes: compute_null_hashes::<H>(self.depth)
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
    storage: HashMap<H::Out, Node<H>>,
    root: &'a mut H::Out,
    root_handle: NodeHash<H>,
    depth: usize,
    recorder: Option<core::cell::RefCell<&'a mut dyn TreeRecorder<H>>>,
    null_hashes: Vec<H::Out>
}

impl<'a, H: Hasher> TreeDBMut<'a, H> {
    pub fn db(&self) -> &dyn HashDB<H, DBValue> {
        self.db
    }

    pub fn db_mut(&mut self) -> &mut dyn HashDB<H, DBValue> {
        self.db
    }


    pub fn lookup(&self, key: &H::Out, depth: usize) -> Result<Node<H>, TreeError> {
        if let Some(node) = self.storage.get(key) {
            return Ok(node.clone());
        }

        let data = if let Some(value) = self.db.get(key, EMPTY_PREFIX) {
            value
        } else {
            if depth == self.depth && key == &self.null_hashes[depth] {
                return Ok(Node::Value(Value::Cached(DBValue::new())));
            } else if key == &self.null_hashes[depth] {
                let null_hash = self.null_hashes[depth + 1];
                return Ok(Node::Inner(
                    NodeHash::Hash(null_hash),
                    NodeHash::Hash(null_hash),
                ));
            } else {
                return Err(TreeError::UnexpectedError);
            }
        };

        let node: Node<H> = data.try_into()?;
        self.recorder
            .as_ref()
            .map(|r| r.borrow_mut().record(node.clone()));

        Ok(node)
    }

    pub fn get(&self, key: &[u8]) -> Result<Node<H>, TreeError> {
        // if index < 1 || (1 << self.depth) * 3 <= index {
        //     return Err(TreeError::IndexOutOfBounds);
        // }
        let mut current_node = self.lookup(self.root_handle.get_hash(), 0)?;

        for (depth, &bit) in key.iter().enumerate() {
            let key = current_node.get_child(bit)?.get_hash();
            current_node = self.lookup(key, depth + 1)?;
        }

        Ok(current_node)
    }

    fn insert_at(
        &mut self,
        current_node: &mut Node<H>,
        key: &[u8],
        value: DBValue,
    ) -> Result<Node<H>, TreeError> {
        if key.len() == 1 {
            let old_leaf = current_node.get_child(key[0])?;
            let old_value = self.lookup(&old_leaf.get_hash(), self.depth)?;
            let new_node = Node::Value(Value::New(value));
            current_node.set_child_hash(key[0], NodeHash::InMemory(new_node.hash()))?;
            self.storage.insert(new_node.hash(), new_node);
            Ok(old_value)
        } else {
            let child_key = current_node.get_child(key[0])?;
            let mut child_node = self.lookup(child_key.get_hash(), self.depth - key.len() + 1)?;
            let old_value = self.insert_at(&mut child_node, &key[1..], value)?;
            current_node.set_child_hash(key[0], NodeHash::InMemory(child_node.hash()))?;
            self.storage.insert(child_node.hash(), child_node);
            Ok(old_value)
        }
    }

    pub fn commit(&mut self) {
        let root_hash = match self.root_handle {
            NodeHash::Hash(_) => return,
            NodeHash::InMemory(h) => h,
        };

        match self.storage.remove(&root_hash) {
            Some(node) => {
                let encoded_node: Vec<u8> = node.clone().into();
                self.db.emplace(root_hash, EMPTY_PREFIX, encoded_node);
                self.commit_child(node);
                *self.root = root_hash;
                self.root_handle = NodeHash::Hash(*self.root)
            }
            None => return,
        }
    }

    fn commit_child(&mut self, node: Node<H>) {
        match node {
            Node::Inner(left, right) => {
                let hashes = vec![left, right];
                for hash in hashes {
                    match hash {
                        NodeHash::Hash(_) => (),
                        NodeHash::InMemory(hash) => match self.storage.remove(&hash) {
                            Some(node) => {
                                let encoded_node: Vec<u8> = node.clone().into();
                                self.db.emplace(hash, EMPTY_PREFIX, encoded_node);

                                if let &Node::Inner(_, _) = &node {
                                    self.commit_child(node)
                                }
                            }
                            None => (),
                        },
                    }
                }
            }
            Node::Value(value) => match value {
                Value::Cached(_) => (),
                Value::New(mut value) => {
                    let hash = H::hash(&value);
                    let mut encoded_node: Vec<u8> = vec![0];
                    encoded_node.append(&mut value);
                    self.db.emplace(hash, EMPTY_PREFIX, value);
                }
            },
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

    fn get_value(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self
            .get(key)
            .map(|node| node.get_value().map(|x| x.get().to_owned()))?;

        data
    }

    fn get_leaf(&self, key: &[u8]) -> Result<H::Out, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self.get(&key[..key.len() - 1]).map(|node| {
            node.get_child(key[key.len() - 1])
                .map(|x| x.get_hash().to_owned())
        })?;

        data
    }

    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let mut proof = Vec::new();
        proof.push((1, self.root.as_ref().to_vec()));

        let mut current_node = self.lookup(self.root_handle.get_hash(), 0)?;

        for (i, &bit) in key.iter().enumerate() {
            let index = indices::compute_index(&key[..i + 1]);
            let left_index = if index % 2 == 0 { index } else { index ^ 1 };

            if let Node::Inner(left, right) = current_node {
                let key = if bit == 0 {
                    left.get_hash()
                } else {
                    right.get_hash()
                };
                current_node = self.lookup(key, i + 1)?;

                proof.extend_from_slice(&[
                    (left_index, left.get_hash().as_ref().to_vec()),
                    (left_index + 1, right.get_hash().as_ref().to_vec()),
                ]);
            } else {
                return Err(TreeError::UnexpectedNodeType);
            }
        }

        proof.push((0, current_node.get_value()?.get().clone()));

        Ok(proof)
    }

    fn insert(&mut self, key: &[u8], value: DBValue) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        };

        let mut root_data: Node<H> = self.lookup(&self.root_handle.get_hash(), 0)?;

        let old_value = self.insert_at(&mut root_data, key, value)?;

        self.storage.insert(root_data.hash(), root_data.clone());

        self.root_handle = NodeHash::InMemory(root_data.hash());

        old_value.get_value().map(|x| x.get().clone())
    }
}
