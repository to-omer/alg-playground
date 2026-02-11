use crate::SortContext;

const GAPS_DESC: [usize; 20] = [
    7_860_916, 3_493_740, 1_552_773, 690_121, 306_720, 136_320, 60_587, 26_928, 11_968, 5_319,
    2_364, 1_051, 701, 301, 132, 57, 23, 10, 4, 1,
];

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    let len = data.len();
    if len < 2 {
        return;
    }

    let ptr = data.as_mut_ptr();
    unsafe {
        for &gap in &GAPS_DESC {
            if gap >= len {
                continue;
            }
            for i in gap..len {
                let x = *ptr.add(i);
                let mut j = i;
                while j >= gap {
                    let prev = *ptr.add(j - gap);
                    if prev <= x {
                        break;
                    }
                    *ptr.add(j) = prev;
                    j -= gap;
                }
                *ptr.add(j) = x;
            }
        }
    }
}
