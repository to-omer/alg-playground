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
- Left-leaning red-black tree (LLRB): https://algs4.cs.princeton.edu/33balanced/RedBlackBST.java.html

## Performance order (max size 256000, local benches)
Note: results can fluctuate significantly across reruns on a shared machine.

- Core workload: avl -> rb -> llrb -> treap -> wbt -> aa -> rbst -> splay -> zip
- Agg workload: wbt -> avl -> rb -> aa -> llrb -> rbst -> treap -> zip -> splay
- Agg_reverse workload: wbt -> avl -> treap -> zip -> rbst -> splay -> aa -> rb -> llrb
- Agg_lazy workload: avl -> wbt -> rb -> llrb -> zip -> treap -> rbst -> aa -> splay
- Full workload: avl -> wbt -> zip -> rbst -> treap -> splay -> aa -> rb -> llrb
