use std::collections::BTreeSet;
use crate::{DBValue, HashDBRef, Hasher, TreeDBBuilder};
use crate::proof::ProofError::ProofGenerationFailed;

#[derive(Debug)]
pub enum ProofError {
    InvalidIndex,
    ProofGenerationFailed,
}

pub fn generate_proof<H: Hasher>(db: &dyn HashDBRef<H, DBValue>, indices: &[usize], root: H::Out, depth: usize,) -> Result<Vec<(usize, DBValue)>, ProofError> {
    let invalid_indices = indices.iter().any(|index| *index >= 2usize.pow(depth as u32 + 1) || *index < 2usize.pow(depth as u32));
    if invalid_indices {
        return Err(ProofError::InvalidIndex)
    }

    let values: Vec<usize> = indices.iter().cloned().filter(|index| *index >= 2usize.pow(depth as u32)).collect();
    let mut leaves: Vec<usize> = indices.iter().cloned().filter(|index| *index < 2usize.pow(depth as u32)).collect();
    leaves.extend(values.iter().map(|index| *index - 2usize.pow(depth as u32)));

    let tree_db = TreeDBBuilder::<H>::new(db, &root, depth).build();
    let proof = tree_db.storage_proof(&leaves.to_vec()).map_err(|_| ProofError::ProofGenerationFailed)?;

    Ok(proof)
}
