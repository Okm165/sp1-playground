use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;

use super::{air::BYTE_MULT_INDICES, ByteChip};
use crate::{air::MachineAir, runtime::ExecutionRecord};

pub const NUM_ROWS: usize = 1 << 16;

impl<F: Field> MachineAir<F> for ByteChip {
    fn name(&self) -> String {
        "Byte".to_string()
    }

    fn shard(&self, input: &ExecutionRecord, outputs: &mut Vec<ExecutionRecord>) {
        outputs[0].byte_lookups = input.byte_lookups.clone();
    }

    fn include(&self, record: &ExecutionRecord) -> bool {
        !record.byte_lookups.is_empty()
    }

    fn generate_trace(&self, record: &mut ExecutionRecord) -> RowMajorMatrix<F> {
        let (mut trace, event_map) = ByteChip::trace_and_map::<F>();

        for (lookup, mult) in record.byte_lookups.iter() {
            let (row, index) = event_map[lookup];

            // Get the column index for the multiplicity.
            let idx = BYTE_MULT_INDICES[index];
            // Update the trace value
            trace.row_mut(row)[idx] += F::from_canonical_usize(*mult);
        }

        trace
    }
}
