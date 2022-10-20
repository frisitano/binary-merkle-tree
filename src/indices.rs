use super::rstd::Vec;

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
