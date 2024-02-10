use std::borrow::BorrowMut;

use crate::cpu::MemoryRecordEnum;
use crate::runtime::ExecutionRecord;
use crate::syscall::precompiles::blake3::compress::columns::NUM_BLAKE3_COMPRESS_INNER_COLS;
use crate::syscall::precompiles::blake3::{Blake3CompressInnerChip, ROUND_COUNT};

use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;

use crate::chip::Chip;

use super::columns::Blake3CompressInnerCols;
use super::{
    G_INDEX, G_INPUT_SIZE, MSG_SCHEDULE, NUM_MSG_WORDS_PER_CALL, NUM_STATE_WORDS_PER_CALL,
    OPERATION_COUNT,
};

impl<F: PrimeField> Chip<F> for Blake3CompressInnerChip {
    fn name(&self) -> String {
        "Blake3CompressInner".to_string()
    }

    fn shard(&self, input: &ExecutionRecord, outputs: &mut Vec<ExecutionRecord>) {
        outputs[0].blake3_compress_inner_events = input.blake3_compress_inner_events.clone();
    }

    fn include(&self, record: &ExecutionRecord) -> bool {
        !record.blake3_compress_inner_events.is_empty()
    }

    fn generate_trace(&self, record: &mut ExecutionRecord) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        let mut new_field_events = Vec::new();

        for i in 0..record.blake3_compress_inner_events.len() {
            let event = record.blake3_compress_inner_events[i];

            let mut clk = event.clk;
            for round in 0..ROUND_COUNT {
                for operation in 0..OPERATION_COUNT {
                    let mut row = [F::zero(); NUM_BLAKE3_COMPRESS_INNER_COLS];
                    let cols: &mut Blake3CompressInnerCols<F> = row.as_mut_slice().borrow_mut();

                    // Assign basic values to the columns.
                    {
                        cols.segment = F::from_canonical_u32(event.shard);
                        cols.clk = F::from_canonical_u32(clk);

                        cols.round_index = F::from_canonical_u32(round as u32);
                        cols.is_round_index_n[round] = F::one();

                        cols.operation_index = F::from_canonical_u32(operation as u32);
                        cols.is_operation_index_n[operation] = F::one();

                        for i in 0..NUM_STATE_WORDS_PER_CALL {
                            cols.state_index[i] = F::from_canonical_usize(G_INDEX[operation][i]);
                        }

                        for i in 0..NUM_MSG_WORDS_PER_CALL {
                            cols.msg_schedule[i] =
                                F::from_canonical_usize(MSG_SCHEDULE[round][2 * operation + i]);
                        }
                    }

                    // Memory columns.
                    {
                        cols.message_ptr = F::from_canonical_u32(event.message_ptr);
                        for i in 0..NUM_MSG_WORDS_PER_CALL {
                            cols.message_reads[i].populate(
                                event.message_reads[round][operation][i],
                                &mut new_field_events,
                            );
                        }

                        cols.state_ptr = F::from_canonical_u32(event.state_ptr);
                        for i in 0..NUM_STATE_WORDS_PER_CALL {
                            cols.state_reads_writes[i].populate(
                                MemoryRecordEnum::Write(event.state_writes[round][operation][i]),
                                &mut new_field_events,
                            );
                        }
                    }

                    // Apply the `g` operation.
                    {
                        let input: [u32; G_INPUT_SIZE] = [
                            event.state_writes[round][operation][0].prev_value,
                            event.state_writes[round][operation][1].prev_value,
                            event.state_writes[round][operation][2].prev_value,
                            event.state_writes[round][operation][3].prev_value,
                            event.message_reads[round][operation][0].value,
                            event.message_reads[round][operation][1].value,
                        ];

                        cols.g.populate(record, input);
                    }

                    clk += 4;

                    cols.is_real = F::one();

                    rows.push(row);
                }
            }
        }

        record.field_events.extend(new_field_events);

        let nb_rows = rows.len();
        let mut padded_nb_rows = nb_rows.next_power_of_two();
        if padded_nb_rows == 2 || padded_nb_rows == 1 {
            padded_nb_rows = 4;
        }

        for _ in nb_rows..padded_nb_rows {
            let mut row = [F::zero(); NUM_BLAKE3_COMPRESS_INNER_COLS];
            let cols: &mut Blake3CompressInnerCols<F> = row.as_mut_slice().borrow_mut();

            // Put this value in this padded row to avoid failing the constraint.
            cols.round_index = F::from_canonical_usize(ROUND_COUNT);

            rows.push(row);
        }

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_BLAKE3_COMPRESS_INNER_COLS,
        )
    }
}
