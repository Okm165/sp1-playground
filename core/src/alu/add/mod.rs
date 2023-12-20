use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};
use p3_air::{Air, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use valida_derive::AlignedBorrow;

use crate::air::{CurtaAirBuilder, Word};

use crate::runtime::{Opcode, Runtime};
use crate::utils::{pad_to_power_of_two, Chip};

pub const NUM_ADD_COLS: usize = size_of::<AddCols<u8>>();

/// The column layout for the chip.
#[derive(AlignedBorrow, Default)]
pub struct AddCols<T> {
    /// The output operand.
    pub a: Word<T>,

    /// The first input operand.
    pub b: Word<T>,

    /// The second input operand.
    pub c: Word<T>,

    /// Trace.
    pub carry: [T; 3],

    /// Selector to know whether this row is enabled.
    pub is_real: T,
}

/// A chip that implements addition for the opcodes ADD and ADDI.
pub struct AddChip;

impl AddChip {
    pub fn new() -> Self {
        Self {}
    }
}

impl<F: PrimeField> Chip<F> for AddChip {
    fn generate_trace(&self, runtime: &mut Runtime) -> RowMajorMatrix<F> {
        // Generate the trace rows for each event.
        let rows = runtime
            .add_events
            .par_iter()
            .map(|event| {
                let mut row = [F::zero(); NUM_ADD_COLS];
                let cols: &mut AddCols<F> = unsafe { transmute(&mut row) };
                let a = event.a.to_le_bytes();
                let b = event.b.to_le_bytes();
                let c = event.c.to_le_bytes();

                let mut carry = [0u8, 0u8, 0u8];
                if (b[0] as u32) + (c[0] as u32) > 255 {
                    carry[0] = 1;
                    cols.carry[0] = F::one();
                }
                if (b[1] as u32) + (c[1] as u32) + (carry[0] as u32) > 255 {
                    carry[1] = 1;
                    cols.carry[1] = F::one();
                }
                if (b[2] as u32) + (c[2] as u32) + (carry[1] as u32) > 255 {
                    carry[2] = 1;
                    cols.carry[2] = F::one();
                }

                cols.a = Word(a.map(F::from_canonical_u8));
                cols.b = Word(b.map(F::from_canonical_u8));
                cols.c = Word(c.map(F::from_canonical_u8));
                cols.is_real = F::one();
                row
            })
            .collect::<Vec<_>>();

        // Convert the trace to a row major matrix.
        let mut trace =
            RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), NUM_ADD_COLS);

        // Pad the trace to a power of two.
        pad_to_power_of_two::<NUM_ADD_COLS, F>(&mut trace.values);

        trace
    }
}

impl<F> BaseAir<F> for AddChip {
    fn width(&self) -> usize {
        NUM_ADD_COLS
    }
}

impl<AB> Air<AB> for AddChip
where
    AB: CurtaAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &AddCols<AB::Var> = main.row_slice(0).borrow();

        let one = AB::F::one();
        let base = AB::F::from_canonical_u32(1 << 8);

        // For each limb, assert that difference between the carried result and the non-carried
        // result is either zero or the base.
        let overflow_0 = local.b[0] + local.c[0] - local.a[0];
        let overflow_1 = local.b[1] + local.c[1] - local.a[1] + local.carry[0];
        let overflow_2 = local.b[2] + local.c[2] - local.a[2] + local.carry[1];
        let overflow_3 = local.b[3] + local.c[3] - local.a[3] + local.carry[2];
        builder.assert_zero(overflow_0.clone() * (overflow_0.clone() - base));
        builder.assert_zero(overflow_1.clone() * (overflow_1.clone() - base));
        builder.assert_zero(overflow_2.clone() * (overflow_2.clone() - base));
        builder.assert_zero(overflow_3.clone() * (overflow_3.clone() - base));

        // If the carry is one, then the overflow must be the base.
        builder.assert_zero(local.carry[0] * (overflow_0.clone() - base.clone()));
        builder.assert_zero(local.carry[1] * (overflow_1.clone() - base.clone()));
        builder.assert_zero(local.carry[2] * (overflow_2.clone() - base.clone()));

        // If the carry is not one, then the overflow must be zero.
        builder.assert_zero((local.carry[0] - one) * overflow_0.clone());
        builder.assert_zero((local.carry[1] - one) * overflow_1.clone());
        builder.assert_zero((local.carry[2] - one) * overflow_2.clone());

        // Assert that the carry is either zero or one.
        builder.assert_bool(local.carry[0]);
        builder.assert_bool(local.carry[1]);
        builder.assert_bool(local.carry[2]);

        // Receive the arguments.
        builder.receive_alu(
            AB::F::from_canonical_u32(Opcode::ADD as u32),
            local.a,
            local.b,
            local.c,
            local.is_real,
        );
    }
}

#[cfg(test)]
mod tests {
    use p3_challenger::DuplexChallenger;
    use p3_dft::Radix2DitParallel;
    use p3_field::Field;

    use p3_baby_bear::BabyBear;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriBasedPcs, FriConfigImpl, FriLdt};
    use p3_keccak::Keccak256Hash;
    use p3_ldt::QuotientMmcs;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_mds::coset_mds::CosetMds;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfigImpl};
    use rand::thread_rng;

    use crate::{
        alu::AluEvent,
        runtime::{Opcode, Runtime},
        utils::Chip,
    };
    use p3_commit::ExtensionMmcs;

    use super::AddChip;

    #[test]
    fn generate_trace() {
        let program = vec![];
        let mut runtime = Runtime::new(program, 0);
        runtime.add_events = vec![AluEvent::new(0, Opcode::ADD, 14, 8, 6)];
        let chip = AddChip::new();
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut runtime);
        println!("{:?}", trace.values)
    }

    #[test]
    fn prove_babybear() {
        type Val = BabyBear;
        type Domain = Val;
        type Challenge = BinomialExtensionField<Val, 4>;
        type PackedChallenge = BinomialExtensionField<<Domain as Field>::Packing, 4>;

        type MyMds = CosetMds<Val, 16>;
        let mds = MyMds::default();

        type Perm = Poseidon2<Val, MyMds, DiffusionMatrixBabybear, 16, 5>;
        let perm = Perm::new_from_rng(8, 22, mds, DiffusionMatrixBabybear, &mut thread_rng());

        type MyHash = SerializingHasher32<Keccak256Hash>;
        let hash = MyHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<Val, MyHash, 2, 8>;
        let compress = MyCompress::new(hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, MyHash, MyCompress, 8>;
        let val_mmcs = ValMmcs::new(hash, compress);

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = DuplexChallenger<Val, Perm, 16>;

        type Quotient = QuotientMmcs<Domain, Challenge, ValMmcs>;
        type MyFriConfig = FriConfigImpl<Val, Challenge, Quotient, ChallengeMmcs, Challenger>;
        let fri_config = MyFriConfig::new(40, challenge_mmcs);
        let ldt = FriLdt { config: fri_config };

        type Pcs = FriBasedPcs<MyFriConfig, ValMmcs, Dft, Challenger>;
        type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

        let pcs = Pcs::new(dft, val_mmcs, ldt);
        let config = StarkConfigImpl::new(pcs);
        let mut challenger = Challenger::new(perm.clone());

        let program = vec![];
        let mut runtime = Runtime::new(program, 0);
        runtime.add_events = vec![AluEvent::new(0, Opcode::ADD, 14, 8, 6)].repeat(1000);
        let chip = AddChip::new();
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut runtime);
        let proof = prove::<MyConfig, _>(&config, &chip, &mut challenger, trace);

        let mut challenger = Challenger::new(perm);
        verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
