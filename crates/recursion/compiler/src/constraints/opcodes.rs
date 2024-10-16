use serde::{Deserialize, Serialize};

/// Operations that can be constrained inside the circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintOpcode {
    ImmV,
    ImmF,
    ImmE,
    AddV,
    AddF,
    AddE,
    AddEF,
    SubV,
    SubF,
    SubE,
    SubEF,
    MulV,
    MulF,
    MulE,
    MulEF,
    DivF,
    DivE,
    DivEF,
    NegV,
    NegF,
    NegE,
    InvV,
    InvF,
    InvE,
    AssertEqV,
    AssertEqF,
    AssertEqE,
    AssertNeF,
    Permute,
    Num2BitsV,
    Num2BitsF,
    SelectV,
    SelectF,
    SelectE,
    Ext2Felt,
    PrintV,
    PrintF,
    PrintE,
    WitnessV,
    WitnessF,
    WitnessE,
    CommitVkeyHash,
    CommitCommittedValuesDigest,
    CircuitFelts2Ext,
    CircuitFelt2Var,
    PermuteBabyBear,
    ReduceE,
}
