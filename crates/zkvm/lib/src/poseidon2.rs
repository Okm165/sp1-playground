use crate::syscall_poseidon2_permute;
use itertools::Itertools;
use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, PrimeField32};

pub const WIDTH: usize = 16;
pub struct Poseidon2<const RATE: usize, const OUT: usize> {}

impl<const RATE: usize, const OUT: usize> Poseidon2<RATE, OUT> {
    pub fn hash_iter<'a, I>(input: I) -> [BabyBear; OUT]
    where
        I: IntoIterator<Item = &'a BabyBear>,
    {
        assert!(RATE < WIDTH);
        assert!(OUT <= WIDTH);

        let mut state = [u32::default(); WIDTH];
        for input_chunk in &input.into_iter().chunks(RATE) {
            let mut ret = [u32::default(); WIDTH];
            state.iter_mut().zip(input_chunk).for_each(|(s, i)| *s = i.as_canonical_u32());
            unsafe {
                syscall_poseidon2_permute(&state as *const _, &mut ret as *mut _);
            }
            state = ret;
        }

        state[..OUT]
            .iter()
            .map(|f| BabyBear::from_canonical_u32(*f))
            .collect::<Vec<BabyBear>>()
            .try_into()
            .unwrap()
    }
}
