pub trait LazyMapMonoid {
    type Key;
    type Agg: Clone;
    type Act: Clone;

    fn agg_unit() -> Self::Agg;
    fn agg_from_key(key: &Self::Key) -> Self::Agg;
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg;

    fn act_unit() -> Self::Act;

    /// Compose `new` after `old`.
    fn act_compose(new: &Self::Act, old: &Self::Act) -> Self::Act;

    fn act_apply_key(key: &Self::Key, act: &Self::Act) -> Self::Key;
    fn act_apply_agg(agg: &Self::Agg, act: &Self::Act, len: usize) -> Self::Agg;
}

pub struct CorePolicy;

impl LazyMapMonoid for CorePolicy {
    type Key = i64;
    type Agg = ();
    type Act = ();

    fn agg_unit() -> Self::Agg {}

    fn agg_from_key(_key: &Self::Key) -> Self::Agg {}

    fn agg_merge(_left: &Self::Agg, _key: &Self::Key, _right: &Self::Agg) -> Self::Agg {}

    fn act_unit() -> Self::Act {}

    fn act_compose(_new: &Self::Act, _old: &Self::Act) -> Self::Act {}

    fn act_apply_key(key: &Self::Key, _act: &Self::Act) -> Self::Key {
        *key
    }

    fn act_apply_agg(_agg: &Self::Agg, _act: &Self::Act, _len: usize) -> Self::Agg {}
}

pub struct RangeSum;

impl LazyMapMonoid for RangeSum {
    type Key = i64;
    type Agg = i64;
    type Act = ();

    fn agg_unit() -> Self::Agg {
        0
    }

    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        left + key + right
    }

    fn act_unit() -> Self::Act {}

    fn act_compose(_new: &Self::Act, _old: &Self::Act) -> Self::Act {}

    fn act_apply_key(key: &Self::Key, _act: &Self::Act) -> Self::Key {
        *key
    }

    fn act_apply_agg(agg: &Self::Agg, _act: &Self::Act, _len: usize) -> Self::Agg {
        *agg
    }
}

pub struct RangeSumRangeAdd;

impl LazyMapMonoid for RangeSumRangeAdd {
    type Key = i64;
    type Agg = i64;
    type Act = i64;

    fn agg_unit() -> Self::Agg {
        0
    }

    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        left + key + right
    }

    fn act_unit() -> Self::Act {
        0
    }

    fn act_compose(new: &Self::Act, old: &Self::Act) -> Self::Act {
        new + old
    }

    fn act_apply_key(key: &Self::Key, act: &Self::Act) -> Self::Key {
        key + act
    }

    fn act_apply_agg(agg: &Self::Agg, act: &Self::Act, len: usize) -> Self::Agg {
        *agg + act * len as i64
    }
}
