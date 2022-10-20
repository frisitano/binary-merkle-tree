use crate::{
    rstd::{convert::From, BTreeSet, Vec},
    Hasher,
};
use hash_db::{AsHashDB, Prefix, EMPTY_PREFIX};
use memory_db::{KeyFunction, MemoryDB};
use std::marker::PhantomData;

pub struct NoopKey<H: Hasher>(PhantomData<H>);

impl<H: Hasher> KeyFunction<H> for NoopKey<H> {
    type Key = Vec<u8>;

    fn key(hash: &H::Out, _prefix: Prefix) -> Vec<u8> {
        hash.as_ref().to_vec()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StorageProof {
    nodes: BTreeSet<Vec<u8>>,
}

impl StorageProof {
    pub fn new(nodes: impl IntoIterator<Item = Vec<u8>>) -> Self {
        StorageProof {
            nodes: BTreeSet::from_iter(nodes),
        }
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
            db.as_hash_db_mut()
                .emplace(H::hash(&node[1..]), EMPTY_PREFIX, node);
        });
        db
    }
}
