use crate::proof::ProofError::ProofGenerationFailed;
use crate::{
    indices::authentication_indices, rstd::Vec, DBValue, HashDBRef, Hasher, TreeDBBuilder,
};

#[derive(Debug)]
pub enum ProofError {
    InvalidIndex,
    ProofGenerationFailed,
    DataNotFound,
}

pub fn generate_proof<H: Hasher>(
    db: &dyn HashDBRef<H, DBValue>,
    indices: &[usize],
    root: H::Out,
    depth: usize,
    compact: bool,
) -> Result<Vec<(usize, DBValue)>, ProofError> {
    let invalid_indices = indices
        .iter()
        .any(|index| *index >= 2usize.pow(depth as u32 + 2) || *index < 2usize.pow(depth as u32));
    if invalid_indices {
        return Err(ProofError::InvalidIndex);
    }

    let values: Vec<usize> = indices
        .iter()
        .cloned()
        .filter(|index| *index >= 2usize.pow(depth as u32 + 1))
        .collect();
    let values_leaves: Vec<usize> = values
        .iter()
        .map(|index| index - 2usize.pow(depth as u32))
        .collect();
    let mut leaves: Vec<usize> = indices
        .iter()
        .cloned()
        .filter(|index| *index < 2usize.pow(depth as u32 + 1))
        .collect();
    leaves.extend(&values_leaves);

    let tree_db = TreeDBBuilder::<H>::new(db, &root, depth).build();

    let mut proof: Vec<(usize, DBValue)> = Vec::new();
    let authentication_indices = authentication_indices(&leaves, compact, depth);

    if compact {
        leaves = leaves
            .into_iter()
            .filter(|x| !values_leaves.contains(x))
            .collect();
    }

    for &index in leaves.iter().chain(&authentication_indices) {
        let data = tree_db.get(index).map_err(|_| ProofError::DataNotFound)?;
        proof.push((index, data))
    }

    for &value_index in values.iter() {
        let data = tree_db
            .get(value_index)
            .map_err(|_| ProofError::DataNotFound)?;
        proof.push((value_index, data));
    }

    Ok(proof)
}
