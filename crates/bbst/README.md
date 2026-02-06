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
Note: results can fluctuate significantly across reruns on a shared machine.

- Core workload: treap -> avl -> rbst -> zip -> wbt -> rb -> aa -> splay
- Agg workload: avl -> wbt -> rb -> aa -> rbst -> treap -> zip -> splay
- Agg_reverse workload: wbt -> avl -> rb -> treap -> rbst -> zip -> aa -> splay
- Agg_lazy workload: avl -> wbt -> rb -> rbst -> treap -> aa -> zip -> splay
- Full workload: wbt -> avl -> rb -> treap -> rbst -> zip -> aa -> splay
