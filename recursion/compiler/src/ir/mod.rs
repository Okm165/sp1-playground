use core::marker::PhantomData;

use alloc::vec::Vec;
use p3_field::{ExtensionField, Field};

mod builder;
mod ops;
mod symbolic;

pub use builder::*;
pub use ops::*;
pub use symbolic::*;

#[derive(Debug, Clone, Copy)]
pub struct Var<N>(pub u32, pub PhantomData<N>);

#[derive(Debug, Clone, Copy)]
pub struct Felt<F>(pub u32, pub PhantomData<F>);

#[derive(Debug, Clone, Copy)]

pub struct Ext<F, EF>(pub u32, pub PhantomData<(F, EF)>);

#[derive(Debug, Clone, Copy)]

pub enum Usize<N> {
    Const(usize),
    Var(Var<N>),
}

impl<N> Usize<N> {
    pub fn value(&self) -> usize {
        match self {
            Usize::Const(c) => *c,
            Usize::Var(_) => panic!("Cannot get the value of a variable"),
        }
    }
}

pub trait Config: Clone {
    type N: Field;
    type F: Field;
    type EF: ExtensionField<Self::F>;
}

#[derive(Debug, Clone)]
pub enum DslIR<C: Config> {
    Imm(Var<C::N>, C::N),
    ImmFelt(Felt<C::F>, C::F),
    ImmExt(Ext<C::F, C::EF>, C::EF),
    AddV(Var<C::N>, Var<C::N>, Var<C::N>),
    AddVI(Var<C::N>, Var<C::N>, C::N),
    AddF(Felt<C::F>, Felt<C::F>, Felt<C::F>),
    AddFI(Felt<C::F>, Felt<C::F>, C::F),
    AddE(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    AddEI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::EF),
    AddEFI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::F),
    AddEFFI(Ext<C::F, C::EF>, Felt<C::F>, C::EF),
    AddEF(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Felt<C::F>),
    MulV(Var<C::N>, Var<C::N>, Var<C::N>),
    MulVI(Var<C::N>, Var<C::N>, C::N),
    MulF(Felt<C::F>, Felt<C::F>, Felt<C::F>),
    MulFI(Felt<C::F>, Felt<C::F>, C::F),
    MulE(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    MulEI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::EF),
    MulEFI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::F),
    MulEF(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Felt<C::F>),
    SubV(Var<C::N>, Var<C::N>, Var<C::N>),
    SubVI(Var<C::N>, Var<C::N>, C::N),
    SubVIN(Var<C::N>, C::N, Var<C::N>),
    SubF(Felt<C::F>, Felt<C::F>, Felt<C::F>),
    SubFI(Felt<C::F>, Felt<C::F>, C::F),
    SubFIN(Felt<C::F>, C::F, Felt<C::F>),
    SubE(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    SubEI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::EF),
    SubEIN(Ext<C::F, C::EF>, C::EF, Ext<C::F, C::EF>),
    SubEFI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::F),
    SubEFIN(Ext<C::F, C::EF>, C::F, Ext<C::F, C::EF>),
    SubEF(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Felt<C::F>),
    DivF(Felt<C::F>, Felt<C::F>, Felt<C::F>),
    DivFI(Felt<C::F>, Felt<C::F>, C::F),
    DivFIN(Felt<C::F>, C::F, Felt<C::F>),
    DivE(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    DivEI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::EF),
    DivEIN(Ext<C::F, C::EF>, C::EF, Ext<C::F, C::EF>),
    DivEFI(Ext<C::F, C::EF>, Ext<C::F, C::EF>, C::F),
    DivEFIN(Ext<C::F, C::EF>, C::F, Ext<C::F, C::EF>),
    DivEF(Ext<C::F, C::EF>, Ext<C::F, C::EF>, Felt<C::F>),
    NegV(Var<C::N>, Var<C::N>),
    NegF(Felt<C::F>, Felt<C::F>),
    NegE(Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    InvV(Var<C::N>, Var<C::N>),
    InvF(Felt<C::F>, Felt<C::F>),
    InvE(Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    For(Usize<C::N>, Usize<C::N>, Var<C::N>, Vec<DslIR<C>>),
    IfEq(Var<C::N>, Var<C::N>, Vec<DslIR<C>>, Vec<DslIR<C>>),
    IfNe(Var<C::N>, Var<C::N>, Vec<DslIR<C>>, Vec<DslIR<C>>),
    IfEqI(Var<C::N>, C::N, Vec<DslIR<C>>, Vec<DslIR<C>>),
    IfNeI(Var<C::N>, C::N, Vec<DslIR<C>>, Vec<DslIR<C>>),
    AssertEqV(Var<C::N>, Var<C::N>),
    AssertNeV(Var<C::N>, Var<C::N>),
    AssertEqF(Felt<C::F>, Felt<C::F>),
    AssertNeF(Felt<C::F>, Felt<C::F>),
    AssertEqE(Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    AssertNeE(Ext<C::F, C::EF>, Ext<C::F, C::EF>),
    AssertEqVI(Var<C::N>, C::N),
    AssertNeVI(Var<C::N>, C::N),
    AssertEqFI(Felt<C::F>, C::F),
    AssertNeFI(Felt<C::F>, C::F),
    AssertEqEI(Ext<C::F, C::EF>, C::EF),
    AssertNeEI(Ext<C::F, C::EF>, C::EF),
}

impl<N> From<Var<N>> for Usize<N> {
    fn from(v: Var<N>) -> Self {
        Usize::Var(v)
    }
}

impl<N> From<usize> for Usize<N> {
    fn from(c: usize) -> Self {
        Usize::Const(c)
    }
}

impl<N> Var<N> {
    pub fn new(id: u32) -> Self {
        Self(id, PhantomData)
    }

    pub fn id(&self) -> String {
        format!("var{}", self.0)
    }
}

impl<F> Felt<F> {
    pub fn new(id: u32) -> Self {
        Self(id, PhantomData)
    }

    pub fn id(&self) -> String {
        format!("felt{}", self.0)
    }
}

impl<F, EF> Ext<F, EF> {
    pub fn new(id: u32) -> Self {
        Self(id, PhantomData)
    }

    pub fn id(&self) -> String {
        format!("ext{}", self.0)
    }
}
