use crate::{rstd::BTreeSet, TreeRecorder};

/// Record node accesses.
pub struct Recorder {
    nodes: BTreeSet<usize>,
}

impl Recorder {
    /// Create a new `Recorder`.
    pub fn new() -> Self {
        Self {
            nodes: BTreeSet::new(),
        }
    }

    /// Drain all visited nodes.
    pub fn drain(&mut self) -> BTreeSet<usize> {
        std::mem::take(&mut self.nodes)
    }
}

impl TreeRecorder for Recorder {
    fn record(&mut self, node: usize) {
        self.nodes.insert(node);
    }
}

pub enum TreeAccess {
    Value(usize),
    Node(usize),
}
