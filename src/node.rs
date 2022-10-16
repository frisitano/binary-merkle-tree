
use std::clone::Clone;

use super::rstd::convert::{From, TryFrom};

use hash_db::Hasher;

use std::vec::Vec;

use super::{DBValue, TreeError};
use serde::{Deserialize, Serialize};


/// Node Enumb
/// Variants include: Value, Leaf, Inner
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EncodedNode {
    Value(DBValue),
    Inner(Vec<u8>, Vec<u8>),
}

impl<H: Hasher> From<Node<H>> for EncodedNode {
    fn from(node: Node<H>) -> Self {
        match node {
            Node::Value(value) => EncodedNode::Value(value.get().to_vec()),
            Node::Inner(left, right) => EncodedNode::Inner(
                left.get_hash().as_ref().to_vec(),
                right.get_hash().as_ref().to_vec(),
            ),
        }
    }
}

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

impl<H: Hasher> TryFrom<EncodedNode> for Node<H> {
    type Error = TreeError;

    fn try_from(encoded: EncodedNode) -> Result<Self, Self::Error> {
        match encoded {
            EncodedNode::Value(value) => Ok(Self::Value(Value::Cached(value))),
            EncodedNode::Inner(left, right) => {
                let left_hash = decode_hash::<H>(&left).ok_or(TreeError::NodeDeserializationFailed)?;
                let right_hash = decode_hash::<H>(&right).ok_or(TreeError::NodeDeserializationFailed)?;
                Ok(Self::Inner(
                    NodeHash::Hash(left_hash),
                    NodeHash::Hash(right_hash),
                ))
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
        } else if bit == 1{
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

impl EncodedNode {
    pub fn hash<H: Hasher>(&self) -> H::Out {
        match self {
            EncodedNode::Inner(left, right) => {
                let mut combined = Vec::with_capacity(H::LENGTH * 2);
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                H::hash(&combined)
            }
            EncodedNode::Value(value) => H::hash(&value),
        }
    }

    pub fn get_inner_node_hash<H: Hasher>(&self, node: u8) -> Result<H::Out, TreeError> {
        match self {
            EncodedNode::Value(_) => Err(TreeError::UnexpectedNodeType),
            EncodedNode::Inner(left, right) => {
                let data = if node == 0 { left } else { right };
                Ok(decode_hash::<H>(data).unwrap())
            }
        }
    }

    pub fn get_inner_node_value(&self, node: u8) -> Result<Vec<u8>, TreeError> {
        match self {
            EncodedNode::Value(_) => Err(TreeError::UnexpectedNodeType),
            EncodedNode::Inner(left, right) => {
                if node == 0 {
                    Ok(left.clone())
                } else {
                    Ok(right.clone())
                }
            }
        }
    }

    pub fn set_inner_node_data(&mut self, node: u8, value: Vec<u8>) -> Result<(), TreeError> {
        match self {
            EncodedNode::Value(_) => Err(TreeError::UnexpectedNodeType),
            EncodedNode::Inner(left, right) => {
                if node == 0 {
                    *left = value;
                } else {
                    *right = value;
                }
                Ok(())
            }
        }
    }

    pub fn get_value(&self) -> Result<Vec<u8>, TreeError> {
        match self {
            EncodedNode::Value(value) => Ok(value.clone()),
            EncodedNode::Inner(_, _) => Err(TreeError::UnexpectedNodeType),
        }
    }
}

pub fn decode_hash<H: Hasher>(data: &[u8]) -> Option<H::Out> {
    if data.len() != H::LENGTH {
        return None;
    }
    let mut hash = H::Out::default();
    hash.as_mut().copy_from_slice(data);
    Some(hash)
}
