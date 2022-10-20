use super::{
    rstd::{
        convert::{From, TryFrom},
        Vec,
    },
    DBValue, Hasher, TreeError,
};

#[derive(Debug)]
pub enum NodeHash<H: Hasher> {
    InMemory(H::Out),
    Hash(H::Out),
}

impl<H: Hasher> NodeHash<H> {
    pub fn get_hash(&self) -> &H::Out {
        match self {
            NodeHash::Hash(hash) => hash,
            NodeHash::InMemory(hash) => hash,
        }
    }
}

impl<H: Hasher> Clone for NodeHash<H> {
    fn clone(&self) -> Self {
        match self {
            NodeHash::Hash(hash) => NodeHash::Hash(hash.clone()),
            NodeHash::InMemory(hash) => NodeHash::InMemory(hash.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Cached(DBValue),
    New(DBValue),
}

impl Value {
    pub fn get(&self) -> &DBValue {
        match self {
            Value::Cached(value) => value,
            Value::New(value) => value,
        }
    }
}

#[derive(Debug)]
pub enum Node<H: Hasher> {
    Value(Value),
    Inner(NodeHash<H>, NodeHash<H>),
}

impl<H: Hasher> Clone for Node<H> {
    fn clone(&self) -> Self {
        match self {
            Node::Value(value) => Node::Value(value.clone()),
            Node::Inner(left, right) => Node::Inner(left.clone(), right.clone()),
        }
    }
}

impl<H: Hasher> TryFrom<Vec<u8>> for Node<H> {
    type Error = TreeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        match value.get(0) {
            Some(0u8) => Ok(Node::Value(Value::Cached(value[1..].to_vec()))),
            Some(1u8) => {
                let left_hash = decode_hash::<H>(&value[1..(H::LENGTH + 1)])?;
                let right_hash = decode_hash::<H>(&value[(H::LENGTH + 1)..])?;
                Ok(Node::Inner(
                    NodeHash::Hash(left_hash),
                    NodeHash::Hash(right_hash),
                ))
            }
            _ => Err(TreeError::NodeDeserializationFailed),
        }
    }
}

impl<H: Hasher> From<Node<H>> for Vec<u8> {
    fn from(node: Node<H>) -> Self {
        match node {
            Node::Value(value) => {
                let value = value.get();
                let mut combined = Vec::with_capacity(value.len() + 1);
                combined.push(0);
                combined.extend_from_slice(value);
                combined
            }
            Node::Inner(left, right) => {
                let mut combined = Vec::with_capacity(1 + H::LENGTH * 2);
                combined.push(1);
                combined.extend_from_slice(left.get_hash().as_ref());
                combined.extend_from_slice(right.get_hash().as_ref());
                combined
            }
        }
    }
}

impl<H: Hasher> Node<H> {
    pub fn hash(&self) -> H::Out {
        match self {
            Node::Value(value) => H::hash(value.get()),
            Node::Inner(left, right) => {
                let mut combined = Vec::with_capacity(H::LENGTH * 2);
                combined.extend_from_slice(left.get_hash().as_ref());
                combined.extend_from_slice(right.get_hash().as_ref());
                H::hash(&combined)
            }
        }
    }

    pub fn get_child(&self, bit: u8) -> Result<&NodeHash<H>, TreeError> {
        if bit == 0 {
            self.get_left_child()
        } else if bit == 1 {
            self.get_right_child()
        } else {
            Err(TreeError::NodeIndexOutOfBounds)
        }
    }

    pub fn get_left_child(&self) -> Result<&NodeHash<H>, TreeError> {
        match self {
            Node::Value(_) => Err(TreeError::UnexpectedNodeType),
            Node::Inner(left, _) => Ok(left),
        }
    }

    pub fn get_right_child(&self) -> Result<&NodeHash<H>, TreeError> {
        match self {
            Node::Value(_) => Err(TreeError::UnexpectedNodeType),
            Node::Inner(_, right) => Ok(right),
        }
    }

    pub fn get_value(&self) -> Result<&Value, TreeError> {
        match self {
            Node::Value(value) => Ok(value),
            Node::Inner(_, _) => Err(TreeError::UnexpectedNodeType),
        }
    }

    pub fn set_child_hash(&mut self, bit: u8, hash: NodeHash<H>) -> Result<H::Out, TreeError> {
        if bit == 0 {
            self.set_left_child_hash(hash)
        } else if bit == 1 {
            self.set_rigth_child_hash(hash)
        } else {
            Err(TreeError::NodeIndexOutOfBounds)
        }
    }

    pub fn set_left_child_hash(&mut self, hash: NodeHash<H>) -> Result<H::Out, TreeError> {
        match self {
            Node::Value(_) => Err(TreeError::UnexpectedNodeType),
            Node::Inner(left, _) => {
                let old = left.get_hash().clone();
                *left = hash;
                Ok(old)
            }
        }
    }

    pub fn set_rigth_child_hash(&mut self, hash: NodeHash<H>) -> Result<H::Out, TreeError> {
        match self {
            Node::Value(_) => Err(TreeError::UnexpectedNodeType),
            Node::Inner(_, right) => {
                let old = right.get_hash().clone();
                *right = hash;
                Ok(old)
            }
        }
    }

    pub fn set_value(&mut self, new_value: Value) -> Result<H::Out, TreeError> {
        match self {
            Node::Value(value) => {
                let old_hash = H::hash(value.get());
                *value = new_value;
                Ok(old_hash)
            }
            Node::Inner(_, _) => Err(TreeError::UnexpectedNodeType),
        }
    }
}

pub fn decode_hash<H: Hasher>(data: &[u8]) -> Result<H::Out, TreeError> {
    if data.len() != H::LENGTH {
        return Err(TreeError::DecodeHashFailed);
    }
    let mut hash = H::Out::default();
    hash.as_mut().copy_from_slice(data);
    Ok(hash)
}
