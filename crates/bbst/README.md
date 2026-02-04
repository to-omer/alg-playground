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

## Performance order (max size 256000)
- Core workload: treap -> wbt -> splay -> avl -> zip -> aa -> rbst -> rb
- Agg workload: treap -> avl -> wbt -> rbst -> zip -> splay -> aa -> rb
- Agg_reverse workload: treap -> avl -> splay -> wbt -> rbst -> zip -> aa -> rb
- Agg_lazy workload: treap -> wbt -> zip -> rbst -> avl -> splay -> aa -> rb
- Full workload: treap -> wbt -> avl -> rbst -> zip -> splay -> aa -> rb
