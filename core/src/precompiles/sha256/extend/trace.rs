use std::mem::transmute;

use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;

use crate::{air::Word, runtime::Segment, utils::Chip};

use super::{ShaExtendChip, ShaExtendCols, NUM_SHA_EXTEND_COLS};

impl<F: PrimeField> Chip<F> for ShaExtendChip {
    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        for i in 0..segment.sha_extend_events.len() {
            let mut event = segment.sha_extend_events[i].clone();
            let w = &mut event.w;
            for j in 0..48usize {
                let mut row = [F::zero(); NUM_SHA_EXTEND_COLS];
                let cols: &mut ShaExtendCols<F> = unsafe { transmute(&mut row) };

                cols.populate_flags(j);
                cols.segment = F::one();
                cols.clk = F::from_canonical_u32(event.clk);
                cols.w_ptr = F::from_canonical_u32(event.w_ptr);

                self.populate_access(
                    &mut cols.w_i_minus_15,
                    w[16 + j - 15],
                    event.w_i_minus_15_records[j],
                );
                self.populate_access(
                    &mut cols.w_i_minus_2,
                    w[16 + j - 2],
                    event.w_i_minus_2_records[j],
                );
                self.populate_access(
                    &mut cols.w_i_minus_16,
                    w[16 + j - 16],
                    event.w_i_minus_16_records[j],
                );
                self.populate_access(
                    &mut cols.w_i_minus_7,
                    w[16 + j - 7],
                    event.w_i_minus_7_records[j],
                );

                // Compute `s0`.
                let w_i_minus_15_rr_7 = cols.w_i_minus_15_rr_7.populate(segment, w[16 + j - 15], 7);
                let w_i_minus_15_rr_18 =
                    cols.w_i_minus_15_rr_18
                        .populate(segment, w[16 + j - 15], 18);
                let w_i_minus_15_rs_3 = cols.w_i_minus_15_rs_3.populate(segment, w[16 + j - 15], 3);
                let s0_intermediate = cols
                    .s0_intermediate
                    .populate(w_i_minus_15_rr_7, w_i_minus_15_rr_18);
                let s0 = cols.s0.populate(s0_intermediate, w_i_minus_15_rs_3);

                // Compute `s1`.
                cols.w_i_minus_2.value = Word::from(w[16 + j - 2]);
                let w_i_minus_2_rr_17 = cols.w_i_minus_2_rr_17.populate(segment, w[16 + j - 2], 17);
                let w_i_minus_2_rr_19 = cols.w_i_minus_2_rr_19.populate(segment, w[16 + j - 2], 19);
                let w_i_minus_2_rs_10 = cols.w_i_minus_2_rs_10.populate(segment, w[16 + j - 2], 10);
                let s1_intermediate = cols
                    .s1_intermediate
                    .populate(w_i_minus_2_rr_17, w_i_minus_2_rr_19);
                let s1 = cols.s1.populate(s1_intermediate, w_i_minus_2_rs_10);

                // Compute `s2`.
                let s2 = cols.s2.populate(w[16 + j - 16], s0, w[16 + j - 7], s1);

                // Write `s2` to `w[i]`.
                w[16 + j] = s2;
                self.populate_access(&mut cols.w_i, w[16 + j], event.w_i_records[j]);

                cols.is_real = F::one();
                rows.push(row);
            }
        }

        let nb_rows = rows.len();
        let mut padded_nb_rows = nb_rows.next_power_of_two();
        if padded_nb_rows == 2 || padded_nb_rows == 1 {
            padded_nb_rows = 4;
        }
        for i in nb_rows..padded_nb_rows {
            let mut row = [F::zero(); NUM_SHA_EXTEND_COLS];
            let cols: &mut ShaExtendCols<F> = unsafe { transmute(&mut row) };
            cols.populate_flags(i);
            rows.push(row);
        }

        // Convert the trace to a row major matrix.
        let trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_SHA_EXTEND_COLS,
        );

        trace
    }
}
