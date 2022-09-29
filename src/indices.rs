use super::rstd::Vec;

/// Is the tree node the left of the right child relative to it's parent.
pub fn is_left_index(index: usize) -> bool {
    index % 2 == 0
}

/// Get the sibling index of a tree node.
pub fn get_sibling_index(index: usize) -> usize {
    if is_left_index(index) {
        return index + 1;
    }
    index - 1
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

/// Find all the node indicies in the authentication path of a set of node indices.
/// Used to generate inclusion proofs.
pub fn authentication_indices(indices: &[usize], depth: usize) -> Vec<usize> {
    let mut authentication_indices = Vec::new();
    let mut layer_indices = indices.to_vec();

    for _ in 0..depth {
        let sibling_indices = sibling_indices(&layer_indices);
        authentication_indices.extend(difference(&sibling_indices, &layer_indices));
        layer_indices = parent_indices(&layer_indices);
    }

    authentication_indices
}

/// Determine leaf index given an offset and a depth.
pub fn storage_leaf_index(offset: usize, depth: usize) -> usize {
    2usize.pow(depth as u32) + offset
}

/// Determine value index given an offset and a depth.
pub fn storage_value_index(index: usize, depth: usize) -> usize {
    2usize.pow((depth + 1) as u32) + index
}

#[test]
fn authentication_indices_test() {
    assert_eq!(authentication_indices(&[9, 11], 3), [8, 10, 3]);
    assert_eq!(authentication_indices(&[10, 15], 3), [11, 14, 4, 6])
}

