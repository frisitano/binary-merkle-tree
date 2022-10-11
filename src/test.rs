// use crate::{generate_proof, indices::authentication_indices, treedb::TreeDBBuilder, treedbmut::TreeDBMut, DBValue, Hasher, Recorder, Tree, TreeMut, EMPTY_PREFIX, hash_node};
use crate::{
    indices::authentication_indices,
    treedb::{TreeDB, TreeDBBuilder},
    Tree, TreeDBMut, TreeDBMutBuilder, EMPTY_PREFIX, TreeMut,
};
use crate::{DBValue, Hasher, EncodedNode};
use std::marker::PhantomData;
use std::slice;

use hash256_std_hasher::Hash256StdHasher;
use hash_db::{AsHashDB, Prefix};
use memory_db::{HashKey, KeyFunction, MemoryDB};
use serde::Serialize;
use sha3::Sha3_256;
use winterfell::crypto::{hashers, Digest};
use winterfell::math::fields::f128;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Sha3;

impl Hasher for Sha3 {
    type Out = [u8; 32];

    type StdHasher = Hash256StdHasher;

    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        use sha3::Digest;
        sha3::Sha3_256::digest(x).into()
    }
}

pub struct NoopKey<H: Hasher>(PhantomData<H>);

impl<H: Hasher> KeyFunction<H> for NoopKey<H> {
    type Key = Vec<u8>;

    fn key(hash: &H::Out, _prefix: Prefix) -> Vec<u8> {
        hash.as_ref().to_vec()
    }
}

fn test_values() -> Vec<u32> {
    let values: Vec<u32> = vec![5, 10, 13, 3, 14, 100, 23, 100];
    values
}

fn build_data() -> (Vec<EncodedNode>, Vec<EncodedNode>, usize, <Sha3 as Hasher>::Out) {
    let depth = 3usize;
    let values: Vec<u32> = test_values();
    let values: Vec<EncodedNode> = values
        .into_iter()
        .map(|x| EncodedNode::Value(x.to_le_bytes().to_vec()))
        .collect();

    let n = values.len();
    let mut nodes: Vec<EncodedNode> = Vec::with_capacity(2 * n);
    unsafe { nodes.set_len(2 * n) }

    nodes[0] = EncodedNode::Value(Vec::new());
    nodes[n..].clone_from_slice(&values);

    let leaf_pairs = unsafe { slice::from_raw_parts(nodes.as_ptr() as *const [EncodedNode; 2], n) };

    for i in (1..n).rev() {
        let left = &leaf_pairs[i][0];
        let right = &leaf_pairs[i][1];
        nodes[i] = EncodedNode::Inner(
            left.hash::<Sha3>().as_ref().to_vec(),
            right.hash::<Sha3>().as_ref().to_vec(),
        );
    }

    let root = nodes[1].hash::<Sha3>();

    (values, nodes, depth, root)
}

fn build_db_mock() -> (
    MemoryDB<Sha3, NoopKey<Sha3>, Vec<u8>>,
    <Sha3 as Hasher>::Out,
    usize,
) {
    let (values, nodes, depth, root) = build_data();
    let mut memory_db = MemoryDB::<Sha3, NoopKey<Sha3>, Vec<u8>>::default();

    for node in values.iter() {
        memory_db.as_hash_db_mut().emplace(
            node.hash::<Sha3>(),
            EMPTY_PREFIX,
            bincode::serialize(node).unwrap(),
        );
    }

    for index in 1..nodes.len() {
        memory_db.as_hash_db_mut().emplace(
            nodes[index].hash::<Sha3>(),
            EMPTY_PREFIX,
            bincode::serialize(&nodes[index]).unwrap(),
        );
    }

    (memory_db, root, depth)
}

#[test]
fn authentication_indices_test() {
    assert_eq!(authentication_indices(&[9, 11], true, 3), [8, 10, 3]);
    assert_eq!(authentication_indices(&[10, 15], true, 3), [11, 14, 4, 6]);
    assert_eq!(
        authentication_indices(&[9, 11], false, 3),
        [8, 10, 4, 5, 2, 3, 1]
    );
}

#[test]
fn test_get_value() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let test_values = test_values();

    let tree_db_builder = TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth);
    let tree_db = tree_db_builder.build();
    let keys: Vec<Vec<u8>> = Vec::from([
        Vec::from([0, 0, 0]),
        Vec::from([0, 0, 1]),
        Vec::from([0, 1, 0]),
        Vec::from([0, 1, 1]),
        Vec::from([1, 0, 0]),
        Vec::from([1, 0, 1]),
        Vec::from([1, 1, 0]),
        Vec::from([1, 1, 1]),
    ]);
    for (value, key) in test_values.iter().zip(&keys) {
        assert_eq!(
            u32::from_le_bytes(tree_db.get_value(key).unwrap().try_into().unwrap()),
            *value
        )
    }

    let tree_db_mut = TreeDBMut::new(&mut memory_db, &mut root, depth);
    for (value, key) in test_values.iter().zip(keys) {
        assert_eq!(
            u32::from_le_bytes(tree_db_mut.get_value(&key).unwrap().try_into().unwrap()),
            *value
        )
    }
}

#[test]
fn test_get_leaf() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let test_values = test_values();

    let tree_db_builder = TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth);
    let tree_db = tree_db_builder.build();

    let keys: Vec<Vec<u8>> = Vec::from([
        Vec::from([0, 0, 0]),
        Vec::from([0, 0, 1]),
        Vec::from([0, 1, 0]),
        Vec::from([0, 1, 1]),
        Vec::from([1, 0, 0]),
        Vec::from([1, 0, 1]),
        Vec::from([1, 1, 0]),
        Vec::from([1, 1, 1]),
    ]);
    for (value, key) in test_values.iter().zip(&keys) {
        let leaf = Sha3::hash(&value.to_le_bytes());
        assert_eq!(tree_db.get_leaf(key).unwrap(), leaf)
    }

    let tree_db_mut = TreeDBMut::<Sha3>::new(&mut memory_db, &mut root, depth);
    for (value, key) in test_values.iter().zip(keys) {
        let leaf = Sha3::hash(&value.to_le_bytes());
        assert_eq!(tree_db_mut.get_leaf(&key).unwrap(), leaf)
    }
}

#[test]
fn test_get_proof() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let test_values = test_values();

    let tree_db_builder = TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth);
    let tree_db = tree_db_builder.build();
    let key = [0, 1, 1];

    let mut expected: Vec<(usize, DBValue)> = Vec::new();
    expected.push((0, test_values[3].to_le_bytes().to_vec()));
    expected.push((1, tree_db.root().as_ref().to_vec()));
    expected.push((
        2,
        tree_db.get(&[]).unwrap().get_inner_node_value(0).unwrap(),
    ));
    expected.push((
        3,
        tree_db.get(&[]).unwrap().get_inner_node_value(1).unwrap(),
    ));
    expected.push((
        4,
        tree_db.get(&[0]).unwrap().get_inner_node_value(0).unwrap(),
    ));
    expected.push((
        5,
        tree_db.get(&[0]).unwrap().get_inner_node_value(1).unwrap(),
    ));
    expected.push((
        10,
        tree_db
            .get(&[0, 1])
            .unwrap()
            .get_inner_node_value(0)
            .unwrap(),
    ));
    expected.push((
        11,
        tree_db
            .get(&[0, 1])
            .unwrap()
            .get_inner_node_value(1)
            .unwrap(),
    ));

    let mut proof = tree_db.get_proof(&key).unwrap();
    proof.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(proof, expected);

    let tree_db_mut = TreeDBMut::<Sha3>::new(&mut memory_db, &mut root, depth);
    let mut proof = tree_db_mut.get_proof(&key).unwrap();
    proof.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(proof, expected);
}

#[test]
fn test_insert_tree_db_mut() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let test_values = test_values();
    let mut tree_db_mut = TreeDBMut::new(&mut memory_db, &mut root, depth.try_into().unwrap());

    let key = Vec::from([0, 0, 0]);
    let new_value = 67u32;

    let old_value = tree_db_mut
        .insert_value(&key, new_value.to_le_bytes().to_vec())
        .unwrap();
    assert_eq!(old_value, test_values[0].to_le_bytes().to_vec());

    let expected_leaf = Sha3::hash(&new_value.to_le_bytes().to_vec());
    assert_eq!(tree_db_mut.get_leaf(&key).unwrap(), expected_leaf);

    let expected_value = new_value.to_le_bytes();
    assert_eq!(tree_db_mut.get_value(&key).unwrap(), expected_value);

    let expected_parent = {
        let mut concat: Vec<u8> = Vec::new();
        concat.append(&mut expected_leaf.to_vec());
        concat.append(&mut tree_db_mut.get_leaf(&[0, 0, 1]).unwrap());
        Sha3::hash(&concat).to_vec()
    };
    assert_eq!(
        tree_db_mut
            .get(&key[..key.len() - 1])
            .unwrap()
            .hash::<Sha3>()
            .as_ref()
            .to_vec(),
        expected_parent
    );

    let expected_grandparent = {
        let mut concat: Vec<u8> = Vec::new();
        concat.append(&mut expected_parent.to_vec());
        concat.append(&mut tree_db_mut.get(&[0, 1]).unwrap().hash::<Sha3>().to_vec());
        Sha3::hash(&concat)
    };
    assert_eq!(
        tree_db_mut.get(&[0]).unwrap().hash::<Sha3>(),
        expected_grandparent
    );

    let expected_root = {
        let mut concat = expected_grandparent.to_vec();
        let mut sibling = tree_db_mut.get(&[1]).unwrap().hash::<Sha3>().to_vec();
        concat.append(&mut sibling);
        Sha3::hash(&concat).to_vec()
    };
    assert_eq!(tree_db_mut.root().to_vec(), expected_root);
}

// #[test]
// fn test_commit_tree_db_mut() {
//     let (mut memory_db, mut root, depth) = build_db_mock();
//     let mut tree_db_mut = TreeDBMut::new(&mut memory_db, &mut root, depth);
//     let new_value = 67u32;
//     let _old_value = tree_db_mut
//         .insert_value(3, new_value.to_le_bytes().to_vec())
//         .unwrap();

//     tree_db_mut.commit();

//     let expected_root: DBValue = vec![
//         221, 139, 96, 63, 186, 15, 51, 124, 240, 238, 232, 94, 45, 200, 201, 221, 210, 128, 67, 14,
//         30, 252, 192, 76, 194, 31, 143, 116, 171, 178, 152, 98,
//     ];
//     let root = tree_db_mut.get(1).unwrap();
//     assert_eq!(root, expected_root);
//     assert_eq!(tree_db_mut.root(), expected_root.as_slice());
// }
//
// #[test]
// fn test_recorder() {
//     let mut recorder = Recorder::new();
//     let (mut memory_db, root, depth) = build_db_mock();
//     let tree_db_builder =
//         TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth).with_recorder(&mut recorder);
//     let tree_db = tree_db_builder.build();
//
//     let _ = tree_db.get_value(1);
//     let _ = tree_db.get_leaf(7);
//     let _ = tree_db.get_proof(5);
//     let _ = tree_db.get(3);
//
//     let recorded_nodes = recorder.drain();
//     let expected_nodes = vec![13, 15, 17];
//     assert_eq!(recorded_nodes, expected_nodes);
// }
//
// #[test]
// fn test_generate_proof_compact() {
//     let (memory_db, root, depth) = build_db_mock();
//     let tree_db_builder = TreeDBBuilder::<Sha3>::new(&memory_db, &root, depth);
//     let tree_db = tree_db_builder.build();
//
//     let mut proof = generate_proof(&memory_db, &[9, 10, 22], root, depth, true).unwrap();
//     proof.sort_by(|x, y| x.0.cmp(&y.0));
//
//     let expected_indices = vec![8, 9, 10, 11, 22, 15, 6];
//     let mut expected: Vec<(usize, DBValue)> = expected_indices
//         .into_iter()
//         .map(|index| (index, tree_db.get(index).unwrap()))
//         .collect();
//     expected.sort_by(|x, y| x.0.cmp(&y.0));
//
//     assert_eq!(proof, expected);
// }
//
// #[test]
// fn test_generate_proof_not_compact() {
//     let (memory_db, root, depth) = build_db_mock();
//     let tree_db_builder = TreeDBBuilder::<Sha3>::new(&memory_db, &root, depth);
//     let tree_db = tree_db_builder.build();
//
//     let mut proof = generate_proof(&memory_db, &[9, 10, 22], root, depth, false).unwrap();
//     proof.sort_by(|x, y| x.0.cmp(&y.0));
//
//     let expected_indices = vec![22, 14, 15, 7, 6, 3, 1, 10, 11, 5, 2, 9, 8, 4];
//     let mut expected: Vec<(usize, DBValue)> = expected_indices
//         .into_iter()
//         .map(|index| (index, tree_db.get(index).unwrap()))
//         .collect();
//     expected.sort_by(|x, y| x.0.cmp(&y.0));
//
//     assert_eq!(proof, expected);
// }
//
// #[test]
// fn test_generate_proof_from_recorder() {
//     let mut recorder = Recorder::new();
//     let (mut memory_db, root, depth) = build_db_mock();
//     let tree_db_builder =
//         TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth).with_recorder(&mut recorder);
//     let tree_db = tree_db_builder.build();
//
//     let expected_indices = vec![22, 14, 15, 7, 6, 3, 1, 10, 11, 5, 2, 9, 8, 4];
//     let mut expected: Vec<(usize, DBValue)> = expected_indices
//         .into_iter()
//         .map(|index| (index, tree_db.get(index).unwrap()))
//         .collect();
//
//     let _ = tree_db.get_value(6);
//     let _ = tree_db.get_leaf(1);
//     let _ = tree_db.get_proof(2);
//
//     let recorded_nodes = recorder.drain();
//
//     let mut proof = generate_proof(&memory_db, &recorded_nodes, root, depth, false).unwrap();
//     proof.sort_by(|x, y| x.0.cmp(&y.0));
//
//     expected.sort_by(|x, y| x.0.cmp(&y.0));
//
//     assert_eq!(proof, expected);
// }
