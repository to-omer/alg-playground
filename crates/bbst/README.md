# bbst

Balanced binary search tree experiments with a shared sequence API.

## Algorithms and references
- Implicit treap: https://cp-algorithms.com/data_structures/treap.html
- Splay tree: https://en.wikipedia.org/wiki/Splay_tree
- Weight-balanced tree (WBT): https://en.wikipedia.org/wiki/Weight-balanced_tree
- Zip tree: https://arxiv.org/abs/1806.06726
- Randomized BST (RBST): https://cp-algorithms.com/data_structures/treap.html
- AA tree: https://en.wikipedia.org/wiki/AA_tree
- AVL tree: https://en.wikipedia.org/wiki/AVL_tree
- Red-black tree: https://en.wikipedia.org/wiki/Red%E2%80%93black_tree

## Performance order (max size 256000, local benches)
- Core workload: treap -> rb -> avl -> wbt -> splay -> zip -> rbst -> aa
- Agg workload: rb -> treap -> rbst -> zip -> wbt -> avl -> splay -> aa
- Agg_reverse workload: treap -> rbst -> zip -> avl -> wbt -> splay -> aa -> rb
- Agg_lazy workload: rb -> treap -> rbst -> zip -> wbt -> avl -> splay -> aa
- Full workload: treap -> zip -> rbst -> wbt -> splay -> avl -> rb -> aa
