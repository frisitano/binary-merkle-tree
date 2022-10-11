use super::rstd::Vec;

/// Is the tree node the left of the right child relative to it's parent.
pub fn is_left_index(index: usize) -> bool {
    index % 2 == 0
}

/// Get the sibling index of a tree node.
pub fn get_sibling_index(index: usize) -> usize {
    index ^ 1
}

/// Get the sibling index's for an array of tree nodes.
pub fn sibling_indices(indices: &[usize]) -> Vec<usize> {
    indices.iter().cloned().map(get_sibling_index).collect()
}

/// Get the parent index.
pub fn parent_index(index: usize) -> usize {
    if is_left_index(index) {
        return index / 2;
    }
    get_sibling_index(index) / 2
}

/// Get the parent index's for an array of tree nodes.
pub fn parent_indices(indices: &[usize]) -> Vec<usize> {
    let mut parents: Vec<usize> = indices.iter().cloned().map(parent_index).collect();
    parents.dedup();
    parents
}

/// Return the difference of two slices.
pub fn difference<T: Clone + PartialEq>(a: &[T], b: &[T]) -> Vec<T> {
    a.iter().cloned().filter(|x| !b.contains(x)).collect()
}

/// Find all the node indices in the authentication path of a set of node indices.
/// Used to generate inclusion proofs.
pub fn authentication_indices(indices: &[usize], compact: bool, depth: usize) -> Vec<usize> {
    let mut authentication_indices = Vec::new();
    let mut layer_indices = indices.to_vec();

    for _ in 0..depth {
        let sibling_indices = sibling_indices(&layer_indices);
        authentication_indices.extend(difference(&sibling_indices, &layer_indices));
        layer_indices = parent_indices(&layer_indices);

        if compact == false {
            authentication_indices.extend(&layer_indices);
        }
    }

    authentication_indices
}

/// Determine leaf index given an offset and a tree depth.
pub fn leaf_index(offset: usize, depth: usize) -> usize {
    (1 << depth) + offset
}

/// Determine value index given an offset and a tree depth.
pub fn value_index(offset: usize, depth: usize) -> usize {
    (1 << (depth + 1)) + offset
}

pub(crate) fn compute_index(key: &[u8]) -> usize {
    let len = key.len();
    let base: usize = 1 << len;
    let multiplier: Vec<usize> = (0..len).rev().map(|x| 1 << x).collect();
    let sum: usize = key
        .iter()
        .zip(multiplier)
        .map(|(x, y)| (*x as usize) * y)
        .sum();
    base + sum
}
