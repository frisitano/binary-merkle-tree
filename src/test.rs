use crate::{
    DBValue, Hasher, Node, NodeHash, Recorder, Tree, TreeDBBuilder, TreeDBMutBuilder, TreeMut,
    Value, EMPTY_PREFIX,
};

use std::marker::PhantomData;
use std::slice;

use hash256_std_hasher::Hash256StdHasher;
use hash_db::{AsHashDB, Prefix};
use memory_db::{KeyFunction, MemoryDB};

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

fn build_data() -> (
    Vec<Node<Sha3>>,
    Vec<Node<Sha3>>,
    usize,
    <Sha3 as Hasher>::Out,
) {
    let depth = 3usize;
    let values: Vec<u32> = test_values();
    let values: Vec<Node<Sha3>> = values
        .into_iter()
        .map(|x| Node::Value(Value::Cached(x.to_le_bytes().to_vec())))
        .collect();

    let n = values.len();
    let mut nodes: Vec<Node<Sha3>> = Vec::with_capacity(2 * n);
    unsafe { nodes.set_len(2 * n) }

    nodes[0] = Node::Value(Value::Cached(Vec::new()));
    nodes[n..].clone_from_slice(&values);

    let leaf_pairs = unsafe { slice::from_raw_parts(nodes.as_ptr() as *const [Node<Sha3>; 2], n) };

    for i in (1..n).rev() {
        let left = &leaf_pairs[i][0];
        let right = &leaf_pairs[i][1];
        nodes[i] = Node::Inner(NodeHash::Hash(left.hash()), NodeHash::Hash(right.hash()));
    }

    let root = nodes[1].hash();

    (values, nodes, depth, root)
}

fn build_db_mock() -> (
    MemoryDB<Sha3, NoopKey<Sha3>, Vec<u8>>,
    <Sha3 as Hasher>::Out,
    usize,
) {
    let (values, nodes, depth, root) = build_data();
    let mut memory_db = MemoryDB::<Sha3, NoopKey<Sha3>, Vec<u8>>::default();

    for node in values.into_iter() {
        let hash = node.hash();
        let encoded_node: Vec<u8> = node.into();
        memory_db
            .as_hash_db_mut()
            .emplace(hash, EMPTY_PREFIX, encoded_node);
    }

    for index in 1..nodes.len() {
        let hash = nodes[index].hash();
        let encoded_node: Vec<u8> = nodes[index].clone().into();
        memory_db
            .as_hash_db_mut()
            .emplace(hash, EMPTY_PREFIX, encoded_node);
    }

    (memory_db, root, depth)
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

    let tree_db_mut = TreeDBMutBuilder::<Sha3>::new(&mut memory_db, &mut root, depth).build();
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

    let tree_db_mut = TreeDBMutBuilder::<Sha3>::new(&mut memory_db, &mut root, depth).build();
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
        tree_db
            .get(&[])
            .unwrap()
            .get_child(0)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));
    expected.push((
        3,
        tree_db
            .get(&[])
            .unwrap()
            .get_child(1)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));
    expected.push((
        4,
        tree_db
            .get(&[0])
            .unwrap()
            .get_child(0)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));
    expected.push((
        5,
        tree_db
            .get(&[0])
            .unwrap()
            .get_child(1)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));
    expected.push((
        10,
        tree_db
            .get(&[0, 1])
            .unwrap()
            .get_child(0)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));
    expected.push((
        11,
        tree_db
            .get(&[0, 1])
            .unwrap()
            .get_child(1)
            .unwrap()
            .get_hash()
            .as_ref()
            .to_vec(),
    ));

    // let mut proof = tree_db.get_proof(&key).unwrap();
    // proof.sort_by(|a, b| a.0.cmp(&b.0));
    // assert_eq!(proof, expected);

    let tree_db_mut = TreeDBMutBuilder::<Sha3>::new(&mut memory_db, &mut root, depth).build();
    let mut proof = tree_db_mut.get_proof(&key).unwrap();
    proof.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(proof, expected);
}

#[test]
fn test_insert_tree_db_mut() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let test_values = test_values();
    let mut tree_db_mut =
        TreeDBMutBuilder::new(&mut memory_db, &mut root, depth.try_into().unwrap()).build();

    let key = Vec::from([0, 0, 0]);
    let new_value = 67u32;

    let old_value = tree_db_mut
        .insert(&key, new_value.to_le_bytes().to_vec())
        .unwrap();
    assert_eq!(old_value, test_values[0].to_le_bytes().to_vec());

    let expected_leaf = Sha3::hash(&new_value.to_le_bytes().to_vec());
    assert_eq!(tree_db_mut.get_leaf(&key).unwrap(), expected_leaf);

    let expected_value = new_value.to_le_bytes();
    assert_eq!(tree_db_mut.get_value(&key).unwrap(), expected_value);

    let expected_parent = {
        let mut concat: Vec<u8> = Vec::new();
        concat.append(&mut expected_leaf.to_vec());
        concat.append(&mut tree_db_mut.get_leaf(&[0, 0, 1]).unwrap().to_vec());
        Sha3::hash(&concat).to_vec()
    };
    assert_eq!(
        tree_db_mut
            .get(&key[..key.len() - 1])
            .unwrap()
            .hash()
            .as_ref()
            .to_vec(),
        expected_parent
    );

    let expected_grandparent = {
        let mut concat: Vec<u8> = Vec::new();
        concat.append(&mut expected_parent.to_vec());
        concat.append(&mut tree_db_mut.get(&[0, 1]).unwrap().hash().to_vec());
        Sha3::hash(&concat)
    };
    assert_eq!(tree_db_mut.get(&[0]).unwrap().hash(), expected_grandparent);

    let expected_root = {
        let mut concat = expected_grandparent.to_vec();
        let mut sibling = tree_db_mut.get(&[1]).unwrap().hash().to_vec();
        concat.append(&mut sibling);
        Sha3::hash(&concat).to_vec()
    };
    assert_eq!(tree_db_mut.root().to_vec(), expected_root);
}

#[test]
fn test_commit_tree_db_mut() {
    let (mut memory_db, mut root, depth) = build_db_mock();
    let mut tree_db_mut = TreeDBMutBuilder::new(&mut memory_db, &mut root, depth).build();
    let new_value = 67u32;
    let new_value_bytes = new_value.to_le_bytes().to_vec();
    let _old_value = tree_db_mut
        .insert(&[0, 1, 1], new_value_bytes.clone())
        .unwrap();

    tree_db_mut.commit();

    let expected_root: DBValue = vec![
        221, 139, 96, 63, 186, 15, 51, 124, 240, 238, 232, 94, 45, 200, 201, 221, 210, 128, 67, 14,
        30, 252, 192, 76, 194, 31, 143, 116, 171, 178, 152, 98,
    ];
    assert_eq!(tree_db_mut.root().to_vec(), expected_root);
    let retrieved_node: Node<Sha3> = memory_db
        .as_hash_db()
        .get(&Sha3::hash(&new_value_bytes), EMPTY_PREFIX)
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(retrieved_node.get_value().unwrap().get(), &new_value_bytes);
}

#[test]
fn test_recorder() {
    let mut recorder = Recorder::new();
    let (mut memory_db, root, depth) = build_db_mock();
    let tree_db_builder =
        TreeDBBuilder::<Sha3>::new(&mut memory_db, &root, depth).with_recorder(&mut recorder);
    let tree_db = tree_db_builder.build();

    let expected_value = tree_db.get_value(&[0, 0, 0]).unwrap();
    let expected_leaf = tree_db.get_leaf(&[0, 1, 0]).unwrap();
    let expected_proof = tree_db.get_proof(&[0, 1, 1]).unwrap();

    let storage_proof = recorder.drain_storage_proof();
    println!("{:?}", storage_proof);
    let proof_db: MemoryDB<Sha3, _, Vec<u8>> = storage_proof.into_memory_db();
    let proof_tree = TreeDBBuilder::<Sha3>::new(&proof_db, &root, depth).build();

    let value = proof_tree.get_value(&[0, 0, 0]).unwrap();
    let leaf = proof_tree.get_leaf(&[0, 1, 0]).unwrap();
    let proof = proof_tree.get_proof(&[0, 1, 1]).unwrap();

    assert_eq!(value, expected_value);
    assert_eq!(leaf, expected_leaf);
    assert_eq!(proof, expected_proof);
}

#[test]
fn test_null_hash() {
    let null_hashes: Vec<<Sha3 as Hasher>::Out> = (0..64)
        .scan(Sha3::hash(&[]), |null_hash, _| {
            let value = *null_hash;
            *null_hash = Sha3::hash(&[null_hash.as_ref(), null_hash.as_ref()].concat());
            Some(value)
        })
        .collect();
    let leaf = Sha3::hash(&[]);
    let concatenated = [leaf.as_ref(), leaf.as_ref()].concat();
    println!("leaf {:?}", leaf);
    println!("concatenated {:?}", concatenated);
    let layer_2 = Sha3::hash(&concatenated);
    let layer_3 = Sha3::hash(&[layer_2.as_ref(), layer_2.as_ref()].concat());
    assert_eq!(null_hashes[0], leaf);
    assert_eq!(null_hashes[1], layer_2);
    assert_eq!(null_hashes[2], layer_3);
}
