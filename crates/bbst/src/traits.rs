use std::ops::RangeBounds;

pub trait SequenceBase {
    type Key;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&mut self, index: usize) -> Option<&Self::Key>;
    fn insert(&mut self, index: usize, key: Self::Key);
    fn remove(&mut self, index: usize) -> Option<Self::Key>;

    fn extend<I: IntoIterator<Item = Self::Key>>(&mut self, iter: I) {
        for value in iter {
            let index = self.len();
            self.insert(index, value);
        }
    }
}

pub trait SequenceSplitMerge: SequenceBase + Sized {
    fn split_at(&mut self, index: usize) -> Self;
    fn merge(&mut self, right: Self);
}

pub trait SequenceAgg: SequenceBase {
    type Agg;
    fn fold<R: RangeBounds<usize>>(&mut self, range: R) -> Self::Agg;
}

pub trait SequenceLazy: SequenceAgg {
    type Act;
    fn update<R: RangeBounds<usize>>(&mut self, range: R, act: Self::Act);
}

pub trait SequenceReverse: SequenceBase {
    fn reverse<R: RangeBounds<usize>>(&mut self, range: R);
}
