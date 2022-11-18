//! Utility functions to interact with sk's Base-16 Modified Merkle Patricia tree ("trie").

mod node_codec;
mod node_header;

use std::{marker::PhantomData, borrow::Borrow};

/// Various re-exports from the `hash-db` crate.
pub use hash_db::{HashDB as HashDBT, EMPTY_PREFIX};
use hash_db::{Hasher, Prefix};
/// Various re-exports from the `memory-db` crate.
pub use memory_db::{prefixed_key, HashKey, KeyFunction, PrefixedKey};
/// The sk format implementation of `NodeCodec`.
pub use node_codec::NodeCodec;
use sp_trie::TrieStream;
/// Trie codec reexport, mainly child trie support
/// for trie compact proof.
pub use trie_db::proof::VerifyError;
use trie_db::proof::{generate_proof, verify_proof};
/// Various re-exports from the `trie-db` crate.
pub use trie_db::{
	nibble_ops,
	node::{NodePlan, ValuePlan},
	CError, DBValue, Query, Recorder, Trie, TrieCache, TrieConfiguration, TrieDBIterator,
	TrieDBKeyIterator, TrieLayout, TrieMut, TrieRecorder,
};

/// substrate trie layout, with external value nodes.
pub struct LayoutV0<H>(PhantomData<H>);

impl<H> TrieLayout for LayoutV0<H>
where
	H: Hasher,
{
	const USE_EXTENSION: bool = false;
	const ALLOW_EMPTY: bool = true;
	const MAX_INLINE_VALUE: Option<u32> = Some(30000); // TODO a ture number

	type Hash = H;
	type Codec = NodeCodec<Self::Hash>;
}

impl<H> TrieConfiguration for LayoutV0<H>
where
	H: Hasher,
{
	fn trie_root<I, A, B>(input: I) -> <Self::Hash as Hasher>::Out
	where
		I: IntoIterator<Item = (A, B)>,
		A: AsRef<[u8]> + Ord,
		B: AsRef<[u8]>,
	{
		trie_root::trie_root_no_extension::<H, TrieStream, _, _, _>(input, Self::MAX_INLINE_VALUE)
	}

	fn trie_root_unhashed<I, A, B>(input: I) -> Vec<u8>
	where
		I: IntoIterator<Item = (A, B)>,
		A: AsRef<[u8]> + Ord,
		B: AsRef<[u8]>,
	{
		trie_root::unhashed_trie_no_extension::<H, TrieStream, _, _, _>(
			input,
			Self::MAX_INLINE_VALUE,
		)
	}

	fn encode_index(input: u32) -> Vec<u8> {
		codec::Encode::encode(&codec::Compact(input))
	}
}

#[cfg(not(feature = "memory-tracker"))]
type MemTracker = memory_db::NoopTracker<trie_db::DBValue>;
#[cfg(feature = "memory-tracker")]
type MemTracker = memory_db::MemCounter<trie_db::DBValue>;

/// TrieDB error over `TrieConfiguration` trait.
pub type TrieError<L> = trie_db::TrieError<TrieHash<L>, CError<L>>;
/// Reexport from `hash_db`, with genericity set for `Hasher` trait.
pub trait AsHashDB<H: Hasher>: hash_db::AsHashDB<H, trie_db::DBValue> {}
impl<H: Hasher, T: hash_db::AsHashDB<H, trie_db::DBValue>> AsHashDB<H> for T {}
/// Reexport from `hash_db`, with genericity set for `Hasher` trait.
pub type HashDB<'a, H> = dyn hash_db::HashDB<H, trie_db::DBValue> + 'a;
/// Reexport from `hash_db`, with genericity set for `Hasher` trait.
/// This uses a `KeyFunction` for prefixing keys internally (avoiding
/// key conflict for non random keys).
pub type PrefixedMemoryDB<H> =
	memory_db::MemoryDB<H, memory_db::PrefixedKey<H>, trie_db::DBValue, MemTracker>;
/// Reexport from `hash_db`, with genericity set for `Hasher` trait.
/// This uses a noops `KeyFunction` (key addressing must be hashed or using
/// an encoding scheme that avoid key conflict).
pub type MemoryDB<H> = memory_db::MemoryDB<H, memory_db::HashKey<H>, trie_db::DBValue, MemTracker>;
/// Reexport from `hash_db`, with genericity set for `Hasher` trait.
pub type GenericMemoryDB<H, KF> = memory_db::MemoryDB<H, KF, trie_db::DBValue, MemTracker>;

/// Persistent trie database read-access interface for the a given hasher.
pub type TrieDB<'a, 'cache, L> = trie_db::TrieDB<'a, 'cache, L>;
/// Builder for creating a [`TrieDB`].
pub type TrieDBBuilder<'a, 'cache, L> = trie_db::TrieDBBuilder<'a, 'cache, L>;
/// Persistent trie database write-access interface for the a given hasher.
pub type TrieDBMut<'a, L> = trie_db::TrieDBMut<'a, L>;
/// Builder for creating a [`TrieDBMut`].
pub type TrieDBMutBuilder<'a, L> = trie_db::TrieDBMutBuilder<'a, L>;
/// Querying interface, as in `trie_db` but less generic.
pub type Lookup<'a, 'cache, L, Q> = trie_db::Lookup<'a, 'cache, L, Q>;
/// Hash type for a trie layout.
pub type TrieHash<L> = <<L as TrieLayout>::Hash as Hasher>::Out;
/// This module is for non generic definition of trie type.
/// Only the `Hasher` trait is generic in this case.
pub mod trie_types {
	use sp_trie::Error;

use super::*;

	/// Persistent trie database read-access interface for the a given hasher.
	///
	/// Read only V1 and V0 are compatible, thus we always use V1.
	pub type TrieDB<'a, 'cache, H> = super::TrieDB<'a, 'cache, LayoutV0<H>>;
	/// Builder for creating a [`TrieDB`].
	pub type TrieDBBuilder<'a, 'cache, H> = super::TrieDBBuilder<'a, 'cache, LayoutV0<H>>;
	/// Persistent trie database write-access interface for the a given hasher.
	pub type TrieDBMutV1<'a, H> = super::TrieDBMut<'a, LayoutV0<H>>;
	/// Builder for creating a [`TrieDBMutV1`].
	pub type TrieDBMutBuilderV1<'a, H> = super::TrieDBMutBuilder<'a, LayoutV0<H>>;
	/// Querying interface, as in `trie_db` but less generic.
	pub type Lookup<'a, 'cache, H, Q> = trie_db::Lookup<'a, 'cache, LayoutV0<H>, Q>;
	/// As in `trie_db`, but less generic, error type for the crate.
	pub type TrieError<H> = trie_db::TrieError<H, Error<H>>;
}

/// Create a proof for a subset of keys in a trie.
///
/// The `keys` may contain any set of keys regardless of each one of them is included
/// in the `db`.
///
/// For a key `K` that is included in the `db` a proof of inclusion is generated.
/// For a key `K` that is not included in the `db` a proof of non-inclusion is generated.
/// These can be later checked in `verify_trie_proof`.
pub fn generate_trie_proof<'a, L, I, K, DB>(
	db: &DB,
	root: TrieHash<L>,
	keys: I,
) -> Result<Vec<Vec<u8>>, Box<TrieError<L>>>
where
	L: TrieConfiguration,
	I: IntoIterator<Item = &'a K>,
	K: 'a + AsRef<[u8]>,
	DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>,
{
	generate_proof::<_, L, _, _>(db, &root, keys)
}

/// Verify a set of key-value pairs against a trie root and a proof.
///
/// Checks a set of keys with optional values for inclusion in the proof that was generated by
/// `generate_trie_proof`.
/// If the value in the pair is supplied (`(key, Some(value))`), this key-value pair will be
/// checked for inclusion in the proof.
/// If the value is omitted (`(key, None)`), this key will be checked for non-inclusion in the
/// proof.
pub fn verify_trie_proof<'a, L, I, K, V>(
	root: &TrieHash<L>,
	proof: &[Vec<u8>],
	items: I,
) -> Result<(), VerifyError<TrieHash<L>, CError<L>>>
where
	L: TrieConfiguration,
	I: IntoIterator<Item = &'a (K, Option<V>)>,
	K: 'a + AsRef<[u8]>,
	V: 'a + AsRef<[u8]>,
{
	verify_proof::<L, _, _, _>(root, proof, items)
}

/// Determine a trie root given a hash DB and delta values.
pub fn delta_trie_root<L: TrieConfiguration, I, A, B, DB, V>(
	db: &mut DB,
	mut root: TrieHash<L>,
	delta: I,
	recorder: Option<&mut dyn trie_db::TrieRecorder<TrieHash<L>>>,
	cache: Option<&mut dyn TrieCache<L::Codec>>,
) -> Result<TrieHash<L>, Box<TrieError<L>>>
where
	I: IntoIterator<Item = (A, B)>,
	A: Borrow<[u8]>,
	B: Borrow<Option<V>>,
	V: Borrow<[u8]>,
	DB: hash_db::HashDB<L::Hash, trie_db::DBValue>,
{
	{
		let mut trie = TrieDBMutBuilder::<L>::from_existing(db, &mut root)
			.with_optional_cache(cache)
			.with_optional_recorder(recorder)
			.build();

		let mut delta = delta.into_iter().collect::<Vec<_>>();
		delta.sort_by(|l, r| l.0.borrow().cmp(r.0.borrow()));

		for (key, change) in delta {
			match change.borrow() {
				Some(val) => trie.insert(key.borrow(), val.borrow())?,
				None => trie.remove(key.borrow())?,
			};
		}
	}

	Ok(root)
}

/// Read a value from the trie.
pub fn read_trie_value<L: TrieLayout, DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>>(
	db: &DB,
	root: &TrieHash<L>,
	key: &[u8],
	recorder: Option<&mut dyn TrieRecorder<TrieHash<L>>>,
	cache: Option<&mut dyn TrieCache<L::Codec>>,
) -> Result<Option<Vec<u8>>, Box<TrieError<L>>> {
	TrieDBBuilder::<L>::new(db, root)
		.with_optional_cache(cache)
		.with_optional_recorder(recorder)
		.build()
		.get(key)
}

/// Read a value from the trie with given Query.
pub fn read_trie_value_with<
	L: TrieLayout,
	Q: Query<L::Hash, Item = Vec<u8>>,
	DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>,
>(
	db: &DB,
	root: &TrieHash<L>,
	key: &[u8],
	query: Q,
) -> Result<Option<Vec<u8>>, Box<TrieError<L>>> {
	TrieDBBuilder::<L>::new(db, root).build().get_with(key, query)
}

/// Determine the empty trie root.
pub fn empty_trie_root<L: TrieConfiguration>() -> <L::Hash as Hasher>::Out {
	L::trie_root::<_, Vec<u8>, Vec<u8>>(core::iter::empty())
}

/// Determine the empty child trie root.
pub fn empty_child_trie_root<L: TrieConfiguration>() -> <L::Hash as Hasher>::Out {
	L::trie_root::<_, Vec<u8>, Vec<u8>>(core::iter::empty())
}

/// Determine a child trie root given its ordered contents, closed form. H is the default hasher,
/// but a generic implementation may ignore this type parameter and use other hashers.
pub fn child_trie_root<L: TrieConfiguration, I, A, B>(input: I) -> <L::Hash as Hasher>::Out
where
	I: IntoIterator<Item = (A, B)>,
	A: AsRef<[u8]> + Ord,
	B: AsRef<[u8]>,
{
	L::trie_root(input)
}

/// Determine a child trie root given a hash DB and delta values. H is the default hasher,
/// but a generic implementation may ignore this type parameter and use other hashers.
pub fn child_delta_trie_root<L: TrieConfiguration, I, A, B, DB, RD, V>(
	keyspace: &[u8],
	db: &mut DB,
	root_data: RD,
	delta: I,
	recorder: Option<&mut dyn TrieRecorder<TrieHash<L>>>,
	cache: Option<&mut dyn TrieCache<L::Codec>>,
) -> Result<<L::Hash as Hasher>::Out, Box<TrieError<L>>>
where
	I: IntoIterator<Item = (A, B)>,
	A: Borrow<[u8]>,
	B: Borrow<Option<V>>,
	V: Borrow<[u8]>,
	RD: AsRef<[u8]>,
	DB: hash_db::HashDB<L::Hash, trie_db::DBValue>,
{
	let mut root = TrieHash::<L>::default();
	// root is fetched from DB, not writable by runtime, so it's always valid.
	root.as_mut().copy_from_slice(root_data.as_ref());

	let mut db = KeySpacedDBMut::new(db, keyspace);
	delta_trie_root::<L, _, _, _, _, _>(&mut db, root, delta, recorder, cache)
}

/// Read a value from the child trie.
pub fn read_child_trie_value<L: TrieConfiguration, DB>(
	keyspace: &[u8],
	db: &DB,
	root: &TrieHash<L>,
	key: &[u8],
	recorder: Option<&mut dyn TrieRecorder<TrieHash<L>>>,
	cache: Option<&mut dyn TrieCache<L::Codec>>,
) -> Result<Option<Vec<u8>>, Box<TrieError<L>>>
where
	DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>,
{
	let db = KeySpacedDB::new(db, keyspace);
	TrieDBBuilder::<L>::new(&db, &root)
		.with_optional_recorder(recorder)
		.with_optional_cache(cache)
		.build()
		.get(key)
		.map(|x| x.map(|val| val.to_vec()))
}

/// Read a hash from the child trie.
pub fn read_child_trie_hash<L: TrieConfiguration, DB>(
	keyspace: &[u8],
	db: &DB,
	root: &TrieHash<L>,
	key: &[u8],
	recorder: Option<&mut dyn TrieRecorder<TrieHash<L>>>,
	cache: Option<&mut dyn TrieCache<L::Codec>>,
) -> Result<Option<TrieHash<L>>, Box<TrieError<L>>>
where
	DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>,
{
	let db = KeySpacedDB::new(db, keyspace);
	TrieDBBuilder::<L>::new(&db, &root)
		.with_optional_recorder(recorder)
		.with_optional_cache(cache)
		.build()
		.get_hash(key)
}

/// Read a value from the child trie with given query.
pub fn read_child_trie_value_with<L, Q, DB>(
	keyspace: &[u8],
	db: &DB,
	root_slice: &[u8],
	key: &[u8],
	query: Q,
) -> Result<Option<Vec<u8>>, Box<TrieError<L>>>
where
	L: TrieConfiguration,
	Q: Query<L::Hash, Item = DBValue>,
	DB: hash_db::HashDBRef<L::Hash, trie_db::DBValue>,
{
	let mut root = TrieHash::<L>::default();
	// root is fetched from DB, not writable by runtime, so it's always valid.
	root.as_mut().copy_from_slice(root_slice);

	let db = KeySpacedDB::new(db, keyspace);
	TrieDBBuilder::<L>::new(&db, &root)
		.build()
		.get_with(key, query)
		.map(|x| x.map(|val| val.to_vec()))
}

/// `HashDB` implementation that append a encoded prefix (unique id bytes) in addition to the
/// prefix of every key value.
pub struct KeySpacedDB<'a, DB: ?Sized, H>(&'a DB, &'a [u8], PhantomData<H>);

/// `HashDBMut` implementation that append a encoded prefix (unique id bytes) in addition to the
/// prefix of every key value.
///
/// Mutable variant of `KeySpacedDB`, see [`KeySpacedDB`].
pub struct KeySpacedDBMut<'a, DB: ?Sized, H>(&'a mut DB, &'a [u8], PhantomData<H>);

/// Utility function used to merge some byte data (keyspace) and `prefix` data
/// before calling key value database primitives.
fn keyspace_as_prefix_alloc(ks: &[u8], prefix: Prefix) -> (Vec<u8>, Option<u8>) {
	let mut result = vec![0; ks.len() + prefix.0.len()];
	result[..ks.len()].copy_from_slice(ks);
	result[ks.len()..].copy_from_slice(prefix.0);
	(result, prefix.1)
}

impl<'a, DB: ?Sized, H> KeySpacedDB<'a, DB, H> {
	/// instantiate new keyspaced db
	pub fn new(db: &'a DB, ks: &'a [u8]) -> Self {
		KeySpacedDB(db, ks, PhantomData)
	}
}

impl<'a, DB: ?Sized, H> KeySpacedDBMut<'a, DB, H> {
	/// instantiate new keyspaced db
	pub fn new(db: &'a mut DB, ks: &'a [u8]) -> Self {
		KeySpacedDBMut(db, ks, PhantomData)
	}
}

impl<'a, DB, H, T> hash_db::HashDBRef<H, T> for KeySpacedDB<'a, DB, H>
where
	DB: hash_db::HashDBRef<H, T> + ?Sized,
	H: Hasher,
	T: From<&'static [u8]>,
{
	fn get(&self, key: &H::Out, prefix: Prefix) -> Option<T> {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.get(key, (&derived_prefix.0, derived_prefix.1))
	}

	fn contains(&self, key: &H::Out, prefix: Prefix) -> bool {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.contains(key, (&derived_prefix.0, derived_prefix.1))
	}
}

impl<'a, DB, H, T> hash_db::HashDB<H, T> for KeySpacedDBMut<'a, DB, H>
where
	DB: hash_db::HashDB<H, T>,
	H: Hasher,
	T: Default + PartialEq<T> + for<'b> From<&'b [u8]> + Clone + Send + Sync,
{
	fn get(&self, key: &H::Out, prefix: Prefix) -> Option<T> {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.get(key, (&derived_prefix.0, derived_prefix.1))
	}

	fn contains(&self, key: &H::Out, prefix: Prefix) -> bool {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.contains(key, (&derived_prefix.0, derived_prefix.1))
	}

	fn insert(&mut self, prefix: Prefix, value: &[u8]) -> H::Out {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.insert((&derived_prefix.0, derived_prefix.1), value)
	}

	fn emplace(&mut self, key: H::Out, prefix: Prefix, value: T) {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.emplace(key, (&derived_prefix.0, derived_prefix.1), value)
	}

	fn remove(&mut self, key: &H::Out, prefix: Prefix) {
		let derived_prefix = keyspace_as_prefix_alloc(self.1, prefix);
		self.0.remove(key, (&derived_prefix.0, derived_prefix.1))
	}
}

impl<'a, DB, H, T> hash_db::AsHashDB<H, T> for KeySpacedDBMut<'a, DB, H>
where
	DB: hash_db::HashDB<H, T>,
	H: Hasher,
	T: Default + PartialEq<T> + for<'b> From<&'b [u8]> + Clone + Send + Sync,
{
	fn as_hash_db(&self) -> &dyn hash_db::HashDB<H, T> {
		self
	}

	fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn hash_db::HashDB<H, T> + 'b) {
		&mut *self
	}
}

/// Constants used into trie simplification codec.
mod trie_constants {
	const FIRST_PREFIX: u8 = 0b_00 << 6;
	pub const LEAF_PREFIX_MASK: u8 = 0b_01 << 6;
	pub const BRANCH_WITHOUT_MASK: u8 = 0b_10 << 6;
	pub const BRANCH_WITH_MASK: u8 = 0b_11 << 6;
	pub const EMPTY_TRIE: u8 = FIRST_PREFIX | (0b_00 << 4);
	pub const ALT_HASHING_LEAF_PREFIX_MASK: u8 = FIRST_PREFIX | (0b_1 << 5);
	pub const ALT_HASHING_BRANCH_WITH_MASK: u8 = FIRST_PREFIX | (0b_01 << 4);
	pub const ESCAPE_COMPACT_HEADER: u8 = EMPTY_TRIE | 0b_00_01;
}