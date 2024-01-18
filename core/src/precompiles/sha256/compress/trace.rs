use std::mem::transmute;

use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;

use crate::{air::Word, runtime::Segment, utils::Chip};

use super::{
    columns::{ShaCompressCols, NUM_SHA_COMPRESS_COLS},
    ShaCompressChip, SHA_COMPRESS_K,
};

impl<F: PrimeField> Chip<F> for ShaCompressChip {
    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        for i in 0..segment.sha_compress_events.len() {
            let mut event = segment.sha_compress_events[i].clone();

            let og_h = event.h.clone();
            let mut v = [0u32; 8].map(Word::from);

            // Load a, b, c, d, e, f, g, h.
            for j in 0..8usize {
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = unsafe { transmute(&mut row) };

                cols.segment = F::one();
                cols.clk = F::from_canonical_u32(event.clk + (j * 4) as u32);
                cols.w_and_h_ptr = F::from_canonical_u32(event.w_and_h_ptr);

                cols.i = F::from_canonical_usize(j);
                cols.octet[j] = F::one();

                self.populate_access(&mut cols.mem, event.h[j], event.h_read_records[j]);
                cols.mem_addr = F::from_canonical_u32(event.w_and_h_ptr + (64 * 4 + j * 4) as u32);

                cols.a = v[0];
                cols.b = v[1];
                cols.c = v[2];
                cols.d = v[3];
                cols.e = v[4];
                cols.f = v[5];
                cols.g = v[6];
                cols.h = v[7];
                match j {
                    0 => cols.a = cols.mem.value,
                    1 => cols.b = cols.mem.value,
                    2 => cols.c = cols.mem.value,
                    3 => cols.d = cols.mem.value,
                    4 => cols.e = cols.mem.value,
                    5 => cols.f = cols.mem.value,
                    6 => cols.g = cols.mem.value,
                    7 => cols.h = cols.mem.value,
                    _ => panic!("unsupported j"),
                };

                v[0] = cols.a;
                v[1] = cols.b;
                v[2] = cols.c;
                v[3] = cols.d;
                v[4] = cols.e;
                v[5] = cols.f;
                v[6] = cols.g;
                v[7] = cols.h;

                cols.is_initialize = F::one();

                rows.push(row);
            }

            // Peforms the compress operation.
            for j in 0..64 {
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = unsafe { transmute(&mut row) };

                cols.segment = F::one();
                cols.clk = F::from_canonical_u32(event.clk + (8 * 4 + j * 4) as u32);
                cols.w_and_h_ptr = F::from_canonical_u32(event.w_and_h_ptr);

                self.populate_access(&mut cols.mem, event.w[j], event.w_i_read_records[j]);
                cols.mem_addr = F::from_canonical_u32(event.w_and_h_ptr + (j * 4) as u32);

                let a = event.h[0];
                let b = event.h[1];
                let c = event.h[2];
                let d = event.h[3];
                let e = event.h[4];
                let f = event.h[5];
                let g = event.h[6];
                let h = event.h[7];
                cols.a = Word::from(a);
                cols.b = Word::from(b);
                cols.c = Word::from(c);
                cols.d = Word::from(d);
                cols.e = Word::from(e);
                cols.f = Word::from(f);
                cols.g = Word::from(g);
                cols.h = Word::from(h);

                let e_rr_6 = cols.e_rr_6.populate(segment, e, 6);
                let e_rr_11 = cols.e_rr_11.populate(segment, e, 11);
                let e_rr_25 = cols.e_rr_25.populate(segment, e, 25);
                let s1_intermeddiate = cols.s1_intermediate.populate(e_rr_6, e_rr_11);
                let s1 = cols.s1.populate(s1_intermeddiate, e_rr_25);

                let e_and_f = cols.e_and_f.populate(e, f);
                let e_not = cols.e_not.populate(e);
                let e_not_and_g = cols.e_not_and_g.populate(e_not, g);
                let ch = cols.ch.populate(e_and_f, e_not_and_g);

                let temp1 = cols
                    .temp1
                    .populate(h, s1, ch, event.w[j], SHA_COMPRESS_K[j]);

                let a_rr_2 = cols.a_rr_2.populate(segment, a, 2);
                let a_rr_13 = cols.a_rr_13.populate(segment, a, 13);
                let a_rr_22 = cols.a_rr_22.populate(segment, a, 22);
                let s0_intermediate = cols.s0_intermediate.populate(a_rr_2, a_rr_13);
                let s0 = cols.s0.populate(s0_intermediate, a_rr_22);

                let a_and_b = cols.a_and_b.populate(a, b);
                let a_and_c = cols.a_and_c.populate(a, c);
                let b_and_c = cols.b_and_c.populate(b, c);
                let maj_intermediate = cols.maj_intermediate.populate(a_and_b, a_and_c);
                let maj = cols.maj.populate(maj_intermediate, b_and_c);

                let temp2 = cols.temp2.populate(s0, maj);

                let d_add_temp1 = cols.d_add_temp1.populate(d, temp1);
                let temp1_add_temp2 = cols.temp1_add_temp2.populate(temp1, temp2);

                event.h[7] = g;
                event.h[6] = f;
                event.h[5] = e;
                event.h[4] = d_add_temp1;
                event.h[3] = c;
                event.h[2] = b;
                event.h[1] = a;
                event.h[0] = temp1_add_temp2;

                cols.is_compression = F::one();
                rows.push(row);
            }

            let mut v: [u32; 8] = (0..8)
                .map(|i| event.h[i])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            // Store a, b, c, d, e, f, g, h.
            for j in 0..8usize {
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = unsafe { transmute(&mut row) };

                cols.segment = F::one();
                cols.clk = F::from_canonical_u32(event.clk + 8 * 4 + 64 * 4 + (j * 4) as u32);
                cols.w_and_h_ptr = F::from_canonical_u32(event.w_and_h_ptr);

                cols.i = F::from_canonical_usize(j);
                cols.octet[j] = F::one();

                self.populate_access(
                    &mut cols.mem,
                    og_h[j].wrapping_add(event.h[j]),
                    event.h_write_records[j],
                );
                cols.mem_addr = F::from_canonical_u32(event.w_and_h_ptr + (64 * 4 + j * 4) as u32);

                v[j] = event.h[j];
                cols.a = Word::from(v[0]);
                cols.b = Word::from(v[1]);
                cols.c = Word::from(v[2]);
                cols.d = Word::from(v[3]);
                cols.e = Word::from(v[4]);
                cols.f = Word::from(v[5]);
                cols.g = Word::from(v[6]);
                cols.h = Word::from(v[7]);

                cols.is_finalize = F::one();
                rows.push(row);
            }
        }

        let nb_rows = rows.len();
        let mut padded_nb_rows = nb_rows.next_power_of_two();
        if padded_nb_rows == 2 || padded_nb_rows == 1 {
            padded_nb_rows = 4;
        }

        for _ in nb_rows..padded_nb_rows {
            let row = [F::zero(); NUM_SHA_COMPRESS_COLS];
            rows.push(row);
        }

        // Convert the trace to a row major matrix.
        let trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_SHA_COMPRESS_COLS,
        );

        trace
    }
}
