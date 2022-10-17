use crate::{TreeRecorder, Node, Hasher, rstd::convert::From, StorageProof, EncodedNode};

/// Record node accesses.
pub struct Recorder<H: Hasher> {
    nodes: Vec<Node<H>>,
}

impl<H: Hasher> Recorder<H> {
    /// Create a new `Recorder`.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }

    /// Drain all visited nodes.
    pub fn drain(&mut self) -> Vec<Node<H>> {
        let nodes = std::mem::take(&mut self.nodes);
        nodes.into_iter().collect()
    }

    pub fn drain_storage_proof(self) -> StorageProof {
        let encoded_nodes: Vec<Vec<u8>> = self.nodes.into_iter().map(|node| {
            let encoded_node: EncodedNode = node.into();
            bincode::serialize(&encoded_node).unwrap()
        }).collect();
        StorageProof::new(encoded_nodes)
    }
}

// impl<H: Hasher> From<Recorder<H>> for 


impl<H: Hasher> TreeRecorder<H> for Recorder<H> {
    fn record(&mut self, node: Node<H>) {
        self.nodes.push(node);
    }
}
