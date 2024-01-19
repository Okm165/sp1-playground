use p3_air::{Air, BaseAir};
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{ProverConstraintFolder, StarkConfig};

use crate::{
    lookup::{Interaction, InteractionBuilder},
    runtime::Segment,
    stark::DebugConstraintBuilder,
};

pub trait Chip<F: Field>: Air<InteractionBuilder<F>> {
    fn name(&self) -> String {
        "".to_string()
    }

    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F>;

    fn receives(&self) -> Vec<Interaction<F>> {
        let mut builder = InteractionBuilder::new(self.width());
        self.eval(&mut builder);
        let (_, receives) = builder.interactions();
        receives
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        let mut builder = InteractionBuilder::new(self.width());
        self.eval(&mut builder);
        let (sends, _) = builder.interactions();
        sends
    }

    fn all_interactions(&self) -> Vec<Interaction<F>> {
        let mut builder = InteractionBuilder::new(self.width());
        self.eval(&mut builder);
        let (mut sends, receives) = builder.interactions();
        sends.extend(receives);
        sends
    }

    /// The width of the permutation trace as a matrix of challenge elements.
    fn permutation_width(&self) -> usize {
        todo!()
    }
}

pub trait AirChip<SC: StarkConfig>:
    Chip<SC::Val>
    + for<'a> Air<ProverConstraintFolder<'a, SC>>
    + for<'a> Air<DebugConstraintBuilder<'a, SC::Val, SC::Challenge>>
{
    fn air_width(&self) -> usize {
        <Self as BaseAir<SC::Val>>::width(self)
    }
}

impl<SC: StarkConfig, T> AirChip<SC> for T where
    T: Chip<SC::Val>
        + for<'a> Air<ProverConstraintFolder<'a, SC>>
        + for<'a> Air<DebugConstraintBuilder<'a, SC::Val, SC::Challenge>>
{
}

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
    if n_real_rows == 0 {
        n_real_rows = 8;
    } else if n_real_rows == 1 {
        n_real_rows = 8;
    }
    values.resize(n_real_rows.next_power_of_two() * N, T::default());
}
