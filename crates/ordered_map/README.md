# ordered_map

Ordered map experiments with multiple balanced BST equivalents.

## Implementations
- Baselines
  - `StdBTreeMap<K,V>`: wrapper of `std::collections::BTreeMap`
  - `SortedVecMap<K,V>`: sorted `Vec<(K,V)>` + binary search
- Comparison models (generic `K: Ord`)
  - `AvlTreeMap<K,V>`
  - `WbtTreeMap<K,V>` (weight-balanced tree)
  - `AaTreeMap<K,V>`
  - `LlrbTreeMap<K,V>` (left-leaning red-black tree)
  - `RbTreeMap<K,V>` (currently a thin wrapper of `LlrbTreeMap` for now)
  - `TreapMap<K,V>`
  - `ZipTreeMap<K,V>`
  - `SplayTreeMap<K,V>`
  - `ScapegoatTreeMap<K,V>`
  - `SkipListMap<K,V>`
  - `BTreeMapCustom<K,V>` (custom B-tree)
- Integer-key specialized (`Key = u64`)
  - `VebMap<V>` (van Emde Boas)
  - `XFastTrieMap<V>`
  - `YFastTrieMap<V>`
  - `FusionTreeMap<V>` (currently a skeleton: high-degree B-tree backbone)

## Algorithms and references
- AVL tree: https://en.wikipedia.org/wiki/AVL_tree
- Red-black tree / LLRB: https://www.cs.princeton.edu/~rs/talks/LLRB/08Dagstuhl/RedBlack.pdf
- AA tree: https://user.it.uu.se/~arneande/ps/simp.pdf
- Weight-balanced tree: https://en.wikipedia.org/wiki/Weight-balanced_tree
- Scapegoat tree: https://people.csail.mit.edu/rivest/pubs/pubs/GR93.pdf
- Treap (randomized search tree): https://faculty.washington.edu/aragon/pubs/rst96.pdf
- Zip tree: https://www.cse.yorku.ca/~andy/courses/4101/references/ZipTrees.pdf
- Splay tree: https://www.cs.cmu.edu/~sleator/papers/self-adjusting.pdf
- Skip list: https://ftp.cs.umd.edu/pub/skipLists/skiplists.pdf
- B-tree: https://doi.org/10.1007/BF00288683
- X-fast / Y-fast trie: https://opendatastructures.org/ods-java.pdf (Chapter 13)
- van Emde Boas tree: https://en.wikipedia.org/wiki/Van_Emde_Boas_tree
- Fusion tree: https://dl.acm.org/doi/10.1145/291891.291901

## Performance order (max size 256000, local benches)
Note: results can fluctuate significantly across reruns on a shared machine.

- Read workload (`get`/`lower_bound`): btree ≈ fusion -> std_btree -> yfast ≈ xfast -> aa ≈ llrb ≈ avl ≈ rb ≈ wbt ≈ veb ≈ scapegoat ≈ splay -> treap ≈ skiplist ≈ zip
- Mixed workload: fusion ≈ btree -> std_btree -> yfast -> aa ≈ avl ≈ veb ≈ rb -> scapegoat ≈ skiplist ≈ llrb ≈ wbt ≈ splay -> treap ≈ zip -> xfast
- Update workload (`insert`/`remove`): btree ≈ fusion ≈ std_btree -> yfast -> splay -> skiplist ≈ veb ≈ avl ≈ scapegoat ≈ wbt ≈ aa ≈ llrb ≈ rb -> treap -> zip -> xfast

`≈` means no statistically significant difference between adjacent implementations (two-sided exact permutation test on 5 reruns' mean estimates; alpha = 0.05).

## Benchmarks

Bench groups:
- `ordered_map/read`: `get` and `lower_bound` only
- `ordered_map/mixed`: mixed `get`/`lower_bound`/`insert`/`remove`
- `ordered_map/update`: `insert` and `remove` only
