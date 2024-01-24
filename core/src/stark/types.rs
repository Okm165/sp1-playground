use p3_commit::{OpenedValues, Pcs};
use p3_matrix::dense::RowMajorMatrix;

use p3_uni_stark::StarkConfig;
use serde::{Deserialize, Serialize};

type Val<SC> = <SC as StarkConfig>::Val;
type OpenningProof<SC> = <<SC as StarkConfig>::Pcs as Pcs<Val<SC>, ValMat<SC>>>::Proof;
pub type OpenningError<SC> = <<SC as StarkConfig>::Pcs as Pcs<Val<SC>, ValMat<SC>>>::Error;
pub type Challenge<SC> = <SC as StarkConfig>::Challenge;
type ValMat<SC> = RowMajorMatrix<Val<SC>>;
type ChallengeMat<SC> = RowMajorMatrix<Challenge<SC>>;
type Com<SC> = <<SC as StarkConfig>::Pcs as Pcs<Val<SC>, ValMat<SC>>>::Commitment;
type PcsProverData<SC> = <<SC as StarkConfig>::Pcs as Pcs<Val<SC>, ValMat<SC>>>::ProverData;

pub type QuotientOpenedValues<T> = Vec<T>;

pub struct SegmentDebugProof<SC: StarkConfig> {
    pub main_commit: Com<SC>,
    pub traces: Vec<ValMat<SC>>,
    pub permutation_traces: Vec<ChallengeMat<SC>>,
}

#[derive(Serialize, Deserialize)]
pub struct MainData<SC: StarkConfig> {
    #[serde(bound(serialize = "ValMat<SC>: Serialize"))]
    #[serde(bound(deserialize = "ValMat<SC>: Deserialize<'de>"))]
    pub traces: Vec<ValMat<SC>>,
    pub main_commit: Com<SC>,
    #[serde(bound(serialize = "PcsProverData<SC>: Serialize"))]
    #[serde(bound(deserialize = "PcsProverData<SC>: Deserialize<'de>"))]
    pub main_data: PcsProverData<SC>,
}

#[derive(Debug, Clone)]
pub struct SegmentCommitment<C> {
    pub main_commit: C,
    pub permutation_commit: C,
    pub quotient_commit: C,
}

#[derive(Debug, Clone)]
pub struct AirOpenedValues<T> {
    pub local: Vec<T>,
    pub next: Vec<T>,
}

#[derive(Debug, Clone)]
pub struct SegmentOpenedValues<T> {
    pub main: Vec<AirOpenedValues<T>>,
    pub permutation: Vec<AirOpenedValues<T>>,
    pub quotient: Vec<QuotientOpenedValues<T>>,
}

pub struct SegmentProof<SC: StarkConfig> {
    pub commitment: SegmentCommitment<Com<SC>>,
    pub opened_values: SegmentOpenedValues<Challenge<SC>>,
    pub commulative_sums: Vec<SC::Challenge>,
    pub openning_proof: OpenningProof<SC>,
    pub degree_bits: Vec<usize>,
}

impl<T> SegmentOpenedValues<T> {
    pub fn into_values(self) -> OpenedValues<T> {
        let Self {
            main,
            permutation,
            quotient,
        } = self;

        let to_values = |values: AirOpenedValues<T>| vec![values.local, values.next];

        vec![
            main.into_iter().map(to_values).collect::<Vec<_>>(),
            permutation.into_iter().map(to_values).collect::<Vec<_>>(),
            quotient.into_iter().map(|v| vec![v]).collect::<Vec<_>>(),
        ]
    }
}
