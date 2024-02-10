mod buffer;
pub mod ec;
pub mod env;
mod logger;
mod poseidon2_instance;
mod programs;
mod prove;
mod tracer;

pub use buffer::*;
pub use logger::*;
pub use prove::*;
pub use tracer::*;

#[cfg(test)]
pub use programs::*;

use crate::{memory::MemoryCols, operations::field::params::Limbs};

pub const fn indices_arr<const N: usize>() -> [usize; N] {
    let mut indices_arr = [0; N];
    let mut i = 0;
    while i < N {
        indices_arr[i] = i;
        i += 1;
    }
    indices_arr
}

pub fn pad_to_power_of_two<const N: usize, T: Clone + Default>(values: &mut Vec<T>) {
    debug_assert!(values.len() % N == 0);
    let mut n_real_rows = values.len() / N;
    if n_real_rows == 0 || n_real_rows == 1 {
        n_real_rows = 8;
    }
    values.resize(n_real_rows.next_power_of_two() * N, T::default());
}

pub fn limbs_from_prev_access<T: Copy, M: MemoryCols<T>>(cols: &[M]) -> Limbs<T> {
    let vec = cols
        .iter()
        .flat_map(|access| access.prev_value().0)
        .collect::<Vec<T>>();

    let sized = vec
        .try_into()
        .unwrap_or_else(|_| panic!("failed to convert to limbs"));
    Limbs(sized)
}

pub fn limbs_from_access<T: Copy, M: MemoryCols<T>>(cols: &[M]) -> Limbs<T> {
    let vec = cols
        .iter()
        .flat_map(|access| access.value().0)
        .collect::<Vec<T>>();

    let sized = vec
        .try_into()
        .unwrap_or_else(|_| panic!("failed to convert to limbs"));
    Limbs(sized)
}

pub fn pad_rows<T: Clone, const N: usize>(rows: &mut Vec<[T; N]>, row_fn: impl Fn() -> [T; N]) {
    let nb_rows = rows.len();
    let mut padded_nb_rows = nb_rows.next_power_of_two();
    if padded_nb_rows == 2 || padded_nb_rows == 1 {
        padded_nb_rows = 4;
    }
    if padded_nb_rows == nb_rows {
        return;
    }
    let dummy_row = row_fn();
    rows.resize(padded_nb_rows, dummy_row);
}

/// Converts a slice of words to a byte array in little endian.
pub fn words_to_bytes_le<const B: usize>(words: &[u32]) -> [u8; B] {
    debug_assert_eq!(words.len() * 4, B);
    words
        .iter()
        .flat_map(|word| word.to_le_bytes().to_vec())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// Converts a byte array in little endian to a slice of words.
pub fn bytes_to_words_le<const W: usize>(bytes: &[u8]) -> [u32; W] {
    debug_assert_eq!(bytes.len(), W * 4);
    bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// Converts a u32 to a string with commas every 3 digits.
pub fn u32_to_comma_separated(value: u32) -> String {
    value
        .to_string()
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect()
}
