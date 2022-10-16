use crate::{TreeRecorder, Node, Hasher};

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
}

impl<H: Hasher> TreeRecorder<H> for Recorder<H> {
    fn record(&mut self, node: Node<H>) {
        self.nodes.push(node);
    }
}
