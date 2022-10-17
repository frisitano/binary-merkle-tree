use crate::{
    indices::authentication_indices, rstd::{Vec, BTreeSet, convert::From}, DBValue, HashDBRef, Hasher, TreeDBBuilder
};
use memory_db::{KeyFunction, MemoryDB};
use std::marker::PhantomData;
use hash_db::{Prefix, AsHashDB, EMPTY_PREFIX};

pub struct NoopKey<H: Hasher>(PhantomData<H>);

impl<H: Hasher> KeyFunction<H> for NoopKey<H> {
    type Key = Vec<u8>;

    fn key(hash: &H::Out, _prefix: Prefix) -> Vec<u8> {
        hash.as_ref().to_vec()
    }
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StorageProof {
    nodes: BTreeSet<Vec<u8>>
}

impl StorageProof {
    pub fn new(nodes: impl IntoIterator<Item = Vec<u8>>) -> Self {
        StorageProof { nodes: BTreeSet::from_iter(nodes) }
    }

    pub fn into_nodes(self) -> BTreeSet<Vec<u8>> {
        self.nodes
    }

    pub fn into_memory_db<H: Hasher>(self) -> MemoryDB<H, NoopKey<H>, Vec<u8>> {
        self.into()
    }
}

impl<H: Hasher> From<StorageProof> for MemoryDB<H, NoopKey<H>, Vec<u8>> {
    fn from(proof: StorageProof) -> Self {
        let mut db = MemoryDB::<H, NoopKey<H>, Vec<u8>>::default();
        proof.into_nodes().into_iter().for_each(|node| {
            db.as_hash_db_mut().emplace(H::hash(&node[1..]), EMPTY_PREFIX, node);
        });
        db
    }
}



// #[derive(Debug)]
// pub enum ProofError {
//     InvalidIndex,
//     ProofGenerationFailed,
//     DataNotFound,
// }

// pub fn generate_proof<H: Hasher>(
//     db: &dyn HashDBRef<H, DBValue>,
//     indices: &[usize],
//     root: H::Out,
//     depth: usize,
//     compact: bool,
// ) -> Result<Vec<(usize, DBValue)>, ProofError> {
//     let invalid_indices = indices
//         .iter()
//         .any(|index| *index >= 2usize.pow(depth as u32 + 2) || *index < 2usize.pow(depth as u32));
//     if invalid_indices {
//         return Err(ProofError::InvalidIndex);
//     }

//     let values: Vec<usize> = indices
//         .iter()
//         .cloned()
//         .filter(|index| *index >= 2usize.pow(depth as u32 + 1))
//         .collect();
//     let values_leaves: Vec<usize> = values
//         .iter()
//         .map(|index| index - 2usize.pow(depth as u32))
//         .collect();
//     let mut leaves: Vec<usize> = indices
//         .iter()
//         .cloned()
//         .filter(|index| *index < 2usize.pow(depth as u32 + 1))
//         .collect();
//     leaves.extend(&values_leaves);

//     let tree_db = TreeDBBuilder::<H>::new(db, &root, depth).build();

//     let mut proof: Vec<(usize, DBValue)> = Vec::new();
//     let authentication_indices = authentication_indices(&leaves, compact, depth);

//     if compact {
//         leaves = leaves
//             .into_iter()
//             .filter(|x| !values_leaves.contains(x))
//             .collect();
//     }

//     for &index in leaves.iter().chain(&authentication_indices) {
//         let data = tree_db.get(index).map_err(|_| ProofError::DataNotFound)?;
//         proof.push((index, data))
//     }

//     for &value_index in values.iter() {
//         let data = tree_db
//             .get(value_index)
//             .map_err(|_| ProofError::DataNotFound)?;
//         proof.push((value_index, data));
//     }

//     Ok(proof)
// }
