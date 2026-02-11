mod algorithms;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataTrack {
    FullU64,
    BoundedU20,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SortAlgorithm {
    InsertionSort,
    BinaryInsertionSort,
    ShellSortCiura,
    HeapSort,
    MergeSortTopDown,
    MergeSortBottomUp,
    NaturalMergeSort,
    Timsort,
    QuickSortMedian3,
    QuickSort3Way,
    DualPivotQuickSort,
    Introsort,
    PdqsortLike,
    BlockQuickSort,
    QuickMergeSort,
    CountingSort,
    PigeonholeSort,
    BucketSort,
    RadixSortLsdBase256,
    AmericanFlagSortMsd,
}

pub const ALL_ALGORITHMS: [SortAlgorithm; 20] = [
    SortAlgorithm::InsertionSort,
    SortAlgorithm::BinaryInsertionSort,
    SortAlgorithm::ShellSortCiura,
    SortAlgorithm::HeapSort,
    SortAlgorithm::MergeSortTopDown,
    SortAlgorithm::MergeSortBottomUp,
    SortAlgorithm::NaturalMergeSort,
    SortAlgorithm::Timsort,
    SortAlgorithm::QuickSortMedian3,
    SortAlgorithm::QuickSort3Way,
    SortAlgorithm::DualPivotQuickSort,
    SortAlgorithm::Introsort,
    SortAlgorithm::PdqsortLike,
    SortAlgorithm::BlockQuickSort,
    SortAlgorithm::QuickMergeSort,
    SortAlgorithm::CountingSort,
    SortAlgorithm::PigeonholeSort,
    SortAlgorithm::BucketSort,
    SortAlgorithm::RadixSortLsdBase256,
    SortAlgorithm::AmericanFlagSortMsd,
];

pub fn all_algorithms() -> &'static [SortAlgorithm] {
    &ALL_ALGORITHMS
}

pub fn algorithm_name(algo: SortAlgorithm) -> &'static str {
    match algo {
        SortAlgorithm::InsertionSort => "insertion_sort",
        SortAlgorithm::BinaryInsertionSort => "binary_insertion_sort",
        SortAlgorithm::ShellSortCiura => "shell_sort_ciura",
        SortAlgorithm::HeapSort => "heap_sort",
        SortAlgorithm::MergeSortTopDown => "merge_sort_top_down",
        SortAlgorithm::MergeSortBottomUp => "merge_sort_bottom_up",
        SortAlgorithm::NaturalMergeSort => "natural_merge_sort",
        SortAlgorithm::Timsort => "timsort",
        SortAlgorithm::QuickSortMedian3 => "quick_sort_median3",
        SortAlgorithm::QuickSort3Way => "quick_sort_3way",
        SortAlgorithm::DualPivotQuickSort => "dual_pivot_quick_sort",
        SortAlgorithm::Introsort => "introsort",
        SortAlgorithm::PdqsortLike => "pdqsort_like",
        SortAlgorithm::BlockQuickSort => "block_quick_sort",
        SortAlgorithm::QuickMergeSort => "quick_merge_sort",
        SortAlgorithm::CountingSort => "counting_sort",
        SortAlgorithm::PigeonholeSort => "pigeonhole_sort",
        SortAlgorithm::BucketSort => "bucket_sort",
        SortAlgorithm::RadixSortLsdBase256 => "radix_sort_lsd_base256",
        SortAlgorithm::AmericanFlagSortMsd => "american_flag_sort_msd",
    }
}

pub fn supports_track(algo: SortAlgorithm, track: DataTrack) -> bool {
    !matches!(
        (algo, track),
        (
            SortAlgorithm::CountingSort | SortAlgorithm::PigeonholeSort,
            DataTrack::FullU64
        )
    )
}

#[derive(Clone, Copy, Debug)]
pub struct TunedParams {
    pub insertion_threshold: usize,
    pub block_partition_size: usize,
    pub introsort_depth_factor_num: usize,
    pub introsort_depth_factor_den: usize,
    pub timsort_min_run: usize,
    pub radix_pass_bits: usize,
    pub bucket_size_divisor: usize,
}

pub const TUNED_PARAMS: TunedParams = TunedParams {
    insertion_threshold: 24,
    block_partition_size: 64,
    introsort_depth_factor_num: 5,
    introsort_depth_factor_den: 2,
    timsort_min_run: 32,
    radix_pass_bits: 8,
    bucket_size_divisor: 32,
};

#[derive(Clone, Debug)]
pub struct SortContext {
    pub scratch: Vec<u64>,
    pub aux: Vec<u64>,
    pub counts256: [usize; 256],
    pub var_counts: Vec<usize>,
}

impl Default for SortContext {
    fn default() -> Self {
        Self {
            scratch: Vec::new(),
            aux: Vec::new(),
            counts256: [0; 256],
            var_counts: Vec::new(),
        }
    }
}

impl SortContext {
    #[inline]
    pub(crate) fn ensure_scratch(&mut self, len: usize) -> &mut [u64] {
        if self.scratch.len() < len {
            self.scratch.resize(len, 0);
        }
        &mut self.scratch[..len]
    }

    #[inline]
    pub(crate) fn ensure_aux(&mut self, len: usize) -> &mut [u64] {
        if self.aux.len() < len {
            self.aux.resize(len, 0);
        }
        &mut self.aux[..len]
    }

    #[inline]
    pub(crate) fn ensure_var_counts(&mut self, len: usize) -> &mut [usize] {
        if self.var_counts.len() < len {
            self.var_counts.resize(len, 0);
        }
        &mut self.var_counts[..len]
    }
}

pub fn sort_u64(algo: SortAlgorithm, data: &mut [u64]) {
    let mut ctx = SortContext::default();
    sort_u64_with_ctx(algo, data, &mut ctx);
}

pub fn sort_u64_with_ctx(algo: SortAlgorithm, data: &mut [u64], ctx: &mut SortContext) {
    match algo {
        SortAlgorithm::InsertionSort => algorithms::insertion_sort::sort(data, ctx),
        SortAlgorithm::BinaryInsertionSort => algorithms::binary_insertion_sort::sort(data, ctx),
        SortAlgorithm::ShellSortCiura => algorithms::shell_sort_ciura::sort(data, ctx),
        SortAlgorithm::HeapSort => algorithms::heap_sort::sort(data, ctx),
        SortAlgorithm::MergeSortTopDown => algorithms::merge_sort_top_down::sort(data, ctx),
        SortAlgorithm::MergeSortBottomUp => algorithms::merge_sort_bottom_up::sort(data, ctx),
        SortAlgorithm::NaturalMergeSort => algorithms::natural_merge_sort::sort(data, ctx),
        SortAlgorithm::Timsort => algorithms::timsort::sort(data, ctx),
        SortAlgorithm::QuickSortMedian3 => algorithms::quick_sort_median3::sort(data, ctx),
        SortAlgorithm::QuickSort3Way => algorithms::quick_sort_3way::sort(data, ctx),
        SortAlgorithm::DualPivotQuickSort => algorithms::dual_pivot_quick_sort::sort(data, ctx),
        SortAlgorithm::Introsort => algorithms::introsort::sort(data, ctx),
        SortAlgorithm::PdqsortLike => algorithms::pdqsort_like::sort(data, ctx),
        SortAlgorithm::BlockQuickSort => algorithms::block_quick_sort::sort(data, ctx),
        SortAlgorithm::QuickMergeSort => algorithms::quick_merge_sort::sort(data, ctx),
        SortAlgorithm::CountingSort => algorithms::counting_sort::sort(data, ctx),
        SortAlgorithm::PigeonholeSort => algorithms::pigeonhole_sort::sort(data, ctx),
        SortAlgorithm::BucketSort => algorithms::bucket_sort::sort(data, ctx),
        SortAlgorithm::RadixSortLsdBase256 => algorithms::radix_sort_lsd_base256::sort(data, ctx),
        SortAlgorithm::AmericanFlagSortMsd => algorithms::american_flag_sort_msd::sort(data, ctx),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use super::*;

    fn assert_sorts_like_std(data: &[u64]) {
        for &algo in all_algorithms() {
            let mut actual = data.to_vec();
            sort_u64(algo, &mut actual);

            let mut expected = data.to_vec();
            expected.sort_unstable();

            assert_eq!(
                actual,
                expected,
                "algorithm={} input_len={}",
                algorithm_name(algo),
                data.len(),
            );
        }
    }

    #[test]
    fn supports_track_contract() {
        for &algo in all_algorithms() {
            match algo {
                SortAlgorithm::CountingSort | SortAlgorithm::PigeonholeSort => {
                    assert!(!supports_track(algo, DataTrack::FullU64));
                    assert!(supports_track(algo, DataTrack::BoundedU20));
                }
                _ => {
                    assert!(supports_track(algo, DataTrack::FullU64));
                    assert!(supports_track(algo, DataTrack::BoundedU20));
                }
            }
        }
    }

    #[test]
    fn algorithm_names_are_unique() {
        let mut seen = HashSet::new();
        for &algo in all_algorithms() {
            assert!(seen.insert(algorithm_name(algo)));
        }
    }

    #[test]
    fn edge_cases() {
        let cases = [
            vec![],
            vec![42],
            vec![1, 2, 3, 4, 5, 6],
            vec![6, 5, 4, 3, 2, 1],
            vec![7; 128],
            vec![u64::MIN, 1, u64::MAX, 0, u64::MAX - 1, 2],
            vec![5, 5, 3, 3, 1, 1, 4, 4, 2, 2, 0, 0],
        ];

        for case in &cases {
            assert_sorts_like_std(case);
        }
    }

    #[test]
    fn fixed_seed_random_cases() {
        let mut rng = StdRng::seed_from_u64(0x5EED_2026);
        for &size in &[2_usize, 3, 8, 31, 32, 63, 64, 127, 128, 511, 2048] {
            let mut data = Vec::with_capacity(size);
            for _ in 0..size {
                data.push(rng.random::<u64>());
            }
            assert_sorts_like_std(&data);
        }
    }

    #[test]
    fn fixed_seed_many_duplicates() {
        let mut rng = StdRng::seed_from_u64(0xD0D1_2026);
        for &size in &[64_usize, 1024, 4096] {
            let mut data = Vec::with_capacity(size);
            for _ in 0..size {
                data.push((rng.random::<u64>() % 16) * 17);
            }
            assert_sorts_like_std(&data);
        }
    }
}
