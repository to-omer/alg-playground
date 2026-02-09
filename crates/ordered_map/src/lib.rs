pub mod impls;

/// Ordered map interface.
///
/// - Keys are unique.
/// - `insert` overwrites the existing value and returns the old one.
/// - `lower_bound` returns the smallest `(k, v)` with `k >= key`.
pub trait OrderedMap {
    type Key: Ord;
    type Value;

    fn new() -> Self;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value>;

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value>;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value>;

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)>;
}

pub use impls::{
    AaTreeMap, AvlTreeMap, BTreeMapCustom, FusionTreeMap, LlrbTreeMap, RbTreeMap, ScapegoatTreeMap,
    SkipListMap, SortedVecMap, SplayTreeMap, StdBTreeMap, TreapMap, VebMap, WbtTreeMap,
    XFastTrieMap, YFastTrieMap, ZipTreeMap,
};

#[cfg(test)]
mod tests {
    use super::OrderedMap;
    use super::{
        AaTreeMap, AvlTreeMap, BTreeMapCustom, FusionTreeMap, LlrbTreeMap, RbTreeMap,
        ScapegoatTreeMap, SkipListMap, SortedVecMap, SplayTreeMap, StdBTreeMap, TreapMap, VebMap,
        WbtTreeMap, XFastTrieMap, YFastTrieMap, ZipTreeMap,
    };
    use std::collections::BTreeMap;

    #[derive(Clone)]
    struct XorShift64 {
        state: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 7;
            x ^= x >> 9;
            x ^= x << 8;
            self.state = x;
            x
        }

        fn gen_usize(&mut self, range: std::ops::Range<usize>) -> usize {
            debug_assert!(range.start < range.end);
            let span = (range.end - range.start) as u64;
            let x = self.next_u64() % span;
            range.start + (x as usize)
        }

        fn gen_u64(&mut self) -> u64 {
            self.next_u64()
        }
    }

    fn oracle_lower_bound(map: &BTreeMap<u64, u64>, key: u64) -> Option<(u64, u64)> {
        map.range(key..).next().map(|(&k, &v)| (k, v))
    }

    fn check_basic<M: OrderedMap<Key = u64, Value = u64>>() {
        let mut map = M::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert_eq!(map.get(&0), None);
        assert_eq!(map.lower_bound(&0), None);
        assert_eq!(map.remove(&0), None);

        assert_eq!(map.insert(1, 10), None);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1).copied(), Some(10));
        assert_eq!(map.lower_bound(&0).map(|(k, v)| (*k, *v)), Some((1, 10)));
        assert_eq!(map.lower_bound(&1).map(|(k, v)| (*k, *v)), Some((1, 10)));
        assert_eq!(map.lower_bound(&2), None);

        assert_eq!(map.insert(1, 99), Some(10));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&1).copied(), Some(99));

        assert_eq!(map.remove(&1), Some(99));
        assert_eq!(map.len(), 0);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.lower_bound(&0), None);
    }

    fn check_bounds_edges<M: OrderedMap<Key = u64, Value = u64>>() {
        let mut map = M::new();
        let keys = [0, 1, u64::MAX - 1, u64::MAX];
        for (i, &k) in keys.iter().enumerate() {
            assert_eq!(map.insert(k, i as u64), None);
        }

        for &query in [0, 1, 2, u64::MAX - 1, u64::MAX].iter() {
            let got = map.lower_bound(&query).map(|(k, v)| (*k, *v));
            let mut oracle = BTreeMap::new();
            for (i, &k) in keys.iter().enumerate() {
                oracle.insert(k, i as u64);
            }
            let expect = oracle_lower_bound(&oracle, query);
            assert_eq!(got, expect, "query={query}");
        }
    }

    fn check_random<M: OrderedMap<Key = u64, Value = u64>>() {
        let mut rng = XorShift64::new(0xDEAD_BEEF_CAFE_BABE);
        let mut map = M::new();
        let mut oracle = BTreeMap::new();

        const OPS: usize = 20_000;
        for _ in 0..OPS {
            let roll = rng.next_u64() % 100;
            let key = rng.gen_u64();
            if roll < 35 {
                let value = rng.gen_u64();
                let got = map.insert(key, value);
                let expect = oracle.insert(key, value);
                assert_eq!(got, expect);
            } else if roll < 55 {
                let got = map.remove(&key);
                let expect = oracle.remove(&key);
                assert_eq!(got, expect);
            } else if roll < 80 {
                let got = map.get(&key).copied();
                let expect = oracle.get(&key).copied();
                assert_eq!(got, expect);
            } else {
                let got = map.lower_bound(&key).map(|(k, v)| (*k, *v));
                let expect = oracle_lower_bound(&oracle, key);
                assert_eq!(got, expect);
            }

            assert_eq!(map.len(), oracle.len());
            if !oracle.is_empty() {
                let any = rng.gen_usize(0..oracle.len());
                let (&ok, &ov) = oracle.iter().nth(any).unwrap();
                assert_eq!(map.get(&ok).copied(), Some(ov));
            }
        }
    }

    macro_rules! test_all {
        ($name:ident, $func:ident) => {
            #[test]
            fn $name() {
                $func::<StdBTreeMap<u64, u64>>();
                $func::<SortedVecMap<u64, u64>>();
                $func::<AvlTreeMap<u64, u64>>();
                $func::<WbtTreeMap<u64, u64>>();
                $func::<AaTreeMap<u64, u64>>();
                $func::<LlrbTreeMap<u64, u64>>();
                $func::<RbTreeMap<u64, u64>>();
                $func::<TreapMap<u64, u64>>();
                $func::<ZipTreeMap<u64, u64>>();
                $func::<SplayTreeMap<u64, u64>>();
                $func::<ScapegoatTreeMap<u64, u64>>();
                $func::<SkipListMap<u64, u64>>();
                $func::<BTreeMapCustom<u64, u64>>();
                $func::<VebMap<u64>>();
                $func::<XFastTrieMap<u64>>();
                $func::<YFastTrieMap<u64>>();
                $func::<FusionTreeMap<u64>>();
            }
        };
    }

    test_all!(basic_all_impls, check_basic);
    test_all!(bounds_edges_all_impls, check_bounds_edges);
    test_all!(random_all_impls, check_random);
}
