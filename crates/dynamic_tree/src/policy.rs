//! Policy (monoid + lazy action) for dynamic tree implementations.
//!
//! This is a lightweight, `Copy`-oriented variant inspired by `bbst`'s policies.

/// A monoid over `Key` with a lazy action `Act`.
///
/// `Agg` represents an aggregate over an *ordered* sequence of keys.
/// Implementations must ensure `key_unit()` behaves as a neutral element in `agg_merge`.
pub trait LazyMapMonoid {
    type Key: Copy;
    type Agg: Copy;
    type Act: Copy;

    /// Whether the aggregate is invariant under sequence reversal.
    ///
    /// If `true`, implementations may skip maintaining reverse aggregates
    /// (`agg_rev`, `path_rev`) for performance.
    const REVERSAL_INVARIANT: bool;

    /// Neutral key that contributes nothing to the aggregate.
    ///
    /// Used for dummy vertices / arc nodes.
    fn key_unit() -> Self::Key;

    fn agg_unit() -> Self::Agg;
    fn agg_from_key(key: &Self::Key) -> Self::Agg;

    /// Merge aggregates as `left + [key] + right`.
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg;

    fn act_unit() -> Self::Act;

    /// Compose actions as `new ∘ old` (apply `old` first, then `new`).
    fn act_compose(new: &Self::Act, old: &Self::Act) -> Self::Act;

    fn act_apply_key(key: &Self::Key, act: &Self::Act) -> Self::Key;

    /// Apply `act` to an aggregate of length `len` (number of affected keys).
    fn act_apply_agg(agg: &Self::Agg, act: &Self::Act, len: usize) -> Self::Agg;
}

#[derive(Clone, Copy, Debug)]
pub enum CorePolicy {}

impl LazyMapMonoid for CorePolicy {
    type Key = i64;
    type Agg = ();
    type Act = ();

    const REVERSAL_INVARIANT: bool = true;

    #[inline(always)]
    fn key_unit() -> Self::Key {
        0
    }

    #[inline(always)]
    fn agg_unit() -> Self::Agg {}

    #[inline(always)]
    fn agg_from_key(_key: &Self::Key) -> Self::Agg {}

    #[inline(always)]
    fn agg_merge(_left: &Self::Agg, _key: &Self::Key, _right: &Self::Agg) -> Self::Agg {}

    #[inline(always)]
    fn act_unit() -> Self::Act {}

    #[inline(always)]
    fn act_compose(_new: &Self::Act, _old: &Self::Act) -> Self::Act {}

    #[inline(always)]
    fn act_apply_key(key: &Self::Key, _act: &Self::Act) -> Self::Key {
        *key
    }

    #[inline(always)]
    fn act_apply_agg(agg: &Self::Agg, _act: &Self::Act, _len: usize) -> Self::Agg {
        *agg
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VertexSum {}

impl LazyMapMonoid for VertexSum {
    type Key = i64;
    type Agg = i64;
    type Act = ();

    const REVERSAL_INVARIANT: bool = true;

    #[inline(always)]
    fn key_unit() -> Self::Key {
        0
    }

    #[inline(always)]
    fn agg_unit() -> Self::Agg {
        0
    }

    #[inline(always)]
    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    #[inline(always)]
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        left.wrapping_add(*key).wrapping_add(*right)
    }

    #[inline(always)]
    fn act_unit() -> Self::Act {}

    #[inline(always)]
    fn act_compose(_new: &Self::Act, _old: &Self::Act) -> Self::Act {}

    #[inline(always)]
    fn act_apply_key(key: &Self::Key, _act: &Self::Act) -> Self::Key {
        *key
    }

    #[inline(always)]
    fn act_apply_agg(agg: &Self::Agg, _act: &Self::Act, _len: usize) -> Self::Agg {
        *agg
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VertexSumAdd {}

impl LazyMapMonoid for VertexSumAdd {
    type Key = i64;
    type Agg = i64;
    type Act = i64;

    const REVERSAL_INVARIANT: bool = true;

    #[inline(always)]
    fn key_unit() -> Self::Key {
        0
    }

    #[inline(always)]
    fn agg_unit() -> Self::Agg {
        0
    }

    #[inline(always)]
    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    #[inline(always)]
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        left.wrapping_add(*key).wrapping_add(*right)
    }

    #[inline(always)]
    fn act_unit() -> Self::Act {
        0
    }

    #[inline(always)]
    fn act_compose(new: &Self::Act, old: &Self::Act) -> Self::Act {
        new.wrapping_add(*old)
    }

    #[inline(always)]
    fn act_apply_key(key: &Self::Key, act: &Self::Act) -> Self::Key {
        key.wrapping_add(*act)
    }

    #[inline(always)]
    fn act_apply_agg(agg: &Self::Agg, act: &Self::Act, len: usize) -> Self::Agg {
        let len = len as i64;
        agg.wrapping_add(act.wrapping_mul(len))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Affine {
    pub a: i64,
    pub b: i64,
}

#[derive(Clone, Copy, Debug)]
pub enum VertexAffineSum {}

impl LazyMapMonoid for VertexAffineSum {
    type Key = i64;
    type Agg = i64;
    type Act = Affine;

    const REVERSAL_INVARIANT: bool = true;

    #[inline(always)]
    fn key_unit() -> Self::Key {
        0
    }

    #[inline(always)]
    fn agg_unit() -> Self::Agg {
        0
    }

    #[inline(always)]
    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    #[inline(always)]
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        left.wrapping_add(*key).wrapping_add(*right)
    }

    #[inline(always)]
    fn act_unit() -> Self::Act {
        Affine { a: 1, b: 0 }
    }

    #[inline(always)]
    fn act_compose(new: &Self::Act, old: &Self::Act) -> Self::Act {
        // new ∘ old
        // a = a_new * a_old
        // b = a_new * b_old + b_new
        Affine {
            a: new.a.wrapping_mul(old.a),
            b: new.a.wrapping_mul(old.b).wrapping_add(new.b),
        }
    }

    #[inline(always)]
    fn act_apply_key(key: &Self::Key, act: &Self::Act) -> Self::Key {
        act.a.wrapping_mul(*key).wrapping_add(act.b)
    }

    #[inline(always)]
    fn act_apply_agg(agg: &Self::Agg, act: &Self::Act, len: usize) -> Self::Agg {
        let len = len as i64;
        act.a
            .wrapping_mul(*agg)
            .wrapping_add(act.b.wrapping_mul(len))
    }
}

#[inline(always)]
fn affine_compose(f: (i64, i64), g: (i64, i64)) -> (i64, i64) {
    // f ∘ g
    // (a_f, b_f) ∘ (a_g, b_g) = (a_f*a_g, a_f*b_g + b_f)
    (
        f.0.wrapping_mul(g.0),
        f.0.wrapping_mul(g.1).wrapping_add(f.1),
    )
}

#[derive(Clone, Copy, Debug)]
pub enum PathComposite {}

impl LazyMapMonoid for PathComposite {
    type Key = (i64, i64);
    type Agg = (i64, i64);
    type Act = ();

    const REVERSAL_INVARIANT: bool = false;

    #[inline(always)]
    fn key_unit() -> Self::Key {
        (1, 0)
    }

    #[inline(always)]
    fn agg_unit() -> Self::Agg {
        (1, 0)
    }

    #[inline(always)]
    fn agg_from_key(key: &Self::Key) -> Self::Agg {
        *key
    }

    #[inline(always)]
    fn agg_merge(left: &Self::Agg, key: &Self::Key, right: &Self::Agg) -> Self::Agg {
        // Sequence: left + [key] + right
        // Composite: right ∘ key ∘ left
        let tmp = affine_compose(*key, *left);
        affine_compose(*right, tmp)
    }

    #[inline(always)]
    fn act_unit() -> Self::Act {}

    #[inline(always)]
    fn act_compose(_new: &Self::Act, _old: &Self::Act) -> Self::Act {}

    #[inline(always)]
    fn act_apply_key(key: &Self::Key, _act: &Self::Act) -> Self::Key {
        *key
    }

    #[inline(always)]
    fn act_apply_agg(agg: &Self::Agg, _act: &Self::Act, _len: usize) -> Self::Agg {
        *agg
    }
}
