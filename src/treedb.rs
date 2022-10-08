use std::thread::current;

use crate::{indices, DBValue, HashDBRef, Hasher, Tree, TreeError, TreeRecorder, EMPTY_PREFIX, Node, decode_hash};

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

    pub fn get(&self, key: &[u8]) -> Result<Node, TreeError> {
        // if index < 1 || (1 << self.depth) * 3 <= index {
        //     return Err(TreeError::IndexOutOfBounds);
        // }
        let root_data = self.db.get(self.root, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)?;

        let mut current_node: Node;
        current_node = bincode::deserialize(&root_data).unwrap();

        for &bit in key {
            if let Node::Inner(left, right) = current_node {
                let key = if bit == 0 { left } else { right };
                let key = decode_hash::<H>(&key).unwrap();
                let data = self.db.get(&key, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)?;
                current_node = bincode::deserialize(&data).unwrap();
            } else {
                return Err(TreeError::UnexpectedNodeType)
            }
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

        self.recorder
            .as_ref()
            .map(|r| r.borrow_mut().record(key));

        data.map(|x| {
            match x {
                Node::Value(value) => Ok(value),
                Node::Inner(_,_) => Err(TreeError::UnexpectedNodeType)
            }
        })?
    }

    fn get_leaf(&self, key: &[u8]) -> Result<DBValue, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let data = self.get(&key[..key.len() - 1]);

        self.recorder
            .as_ref()
            .map(|r| r.borrow_mut().record(key));

        data.map(|x| {
            match x {
                Node::Inner(left,right) => {
                    if key[key.len()-1] == 0 {
                        Ok(left)
                    } else {
                        Ok(right)
                    }
                },
                Node::Value(_) => Err(TreeError::UnexpectedNodeType)
            }
        })?
    }

    fn get_proof(&self, key: &[u8]) -> Result<Vec<(usize, DBValue)>, TreeError> {
        if key.len() != self.depth {
            return Err(TreeError::IndexOutOfBounds);
        }

        let mut proof = Vec::new();
        proof.push((1, self.root.as_ref().to_vec()));

        let root_data = self.db.get(self.root, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)?;
        
        let mut current_node: Node;
        current_node = bincode::deserialize(&root_data).unwrap();

        for (i, &bit) in key.iter().enumerate() {
            let index = compute_index(&key[..i+1]);
            let left_index = if index % 2 == 0 { index } else { index ^ 1 };
            println!("index: {}", index);
            if let Node::Inner(left, right) = current_node {
                let hash_left  = decode_hash::<H>(&left).unwrap();
                let hash_right = decode_hash::<H>(&right).unwrap();
                let key = if bit == 0 { hash_left } else { hash_right };
                let data = self.db.get(&key, EMPTY_PREFIX).ok_or(TreeError::DataNotFound)?;
                current_node = bincode::deserialize(&data).unwrap();

                proof.extend_from_slice(&[(left_index, hash_left.as_ref().to_vec()), (left_index + 1, hash_right.as_ref().to_vec())]);
            } else {
                return Err(TreeError::UnexpectedNodeType)
            }
        }

        self.recorder
            .as_ref()
            .map(|r| r.borrow_mut().record(key));

        Ok(proof)
    }
}

fn compute_index(key: &[u8]) -> usize {
    let len = key.len();
    let base: usize = 1 << len;
    println!("len {} base {}", len, base);
    let multiplier: Vec<usize> = (0..len).rev().map(|x| 1 << x).collect();
    let sum: usize = key.iter().zip(multiplier).map(|(x, y)| (*x as usize) * y).sum();
    base + sum
}