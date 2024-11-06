use super::{
    columns::{Byte3MultCols, Byte3PreprocessedCols, NUM_BYTE3_MULT_COLS},
    Byte3Chip,
};
use core::borrow::Borrow;
use p3_air::{Air, BaseAir, PairBuilder};
use p3_field::Field;
use p3_matrix::Matrix;
use sp1_core_executor::Byte3Opcode;
use sp1_stark::air::SP1AirBuilder;

impl<F: Field> BaseAir<F> for Byte3Chip<F> {
    fn width(&self) -> usize {
        NUM_BYTE3_MULT_COLS
    }
}

impl<AB: SP1AirBuilder + PairBuilder> Air<AB> for Byte3Chip<AB::F> {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local_mult = main.row_slice(0);
        let local_mult: &Byte3MultCols<AB::Var> = (*local_mult).borrow();

        let prep = builder.preprocessed();
        let prep = prep.row_slice(0);
        let local: &Byte3PreprocessedCols<AB::Var> = (*prep).borrow();

        // Send all the lookups for each operation.
        for (i, opcode) in Byte3Opcode::all().iter().enumerate() {
            let field_op = opcode.as_field::<AB::F>();
            let mult = local_mult.multiplicities[i];
            match opcode {
                Byte3Opcode::XOR3 => builder
                    .receive_byte_triple(field_op, local.a, local.b, local.c, local.xor3, mult),
                Byte3Opcode::CH => {
                    builder.receive_byte_triple(field_op, local.a, local.b, local.c, local.ch, mult)
                }
                Byte3Opcode::MAJ => builder
                    .receive_byte_triple(field_op, local.a, local.b, local.c, local.maj, mult),
            }
        }
    }
}
