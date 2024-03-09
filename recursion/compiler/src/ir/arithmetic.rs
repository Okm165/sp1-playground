use crate::asm::AsmInstruction;
use crate::ir::*;
use alloc::rc::Rc;
use core::ops::{Add, Div, Mul, Neg, Sub};
use p3_field::AbstractField;

#[derive(Debug, Clone)]
pub enum Symbolic<F> {
    Const(F),
    Value(Felt<F>),
    Add(Rc<Symbolic<F>>, Rc<Symbolic<F>>),
    Mul(Rc<Symbolic<F>>, Rc<Symbolic<F>>),
    Sub(Rc<Symbolic<F>>, Rc<Symbolic<F>>),
    Div(Rc<Symbolic<F>>, Rc<Symbolic<F>>),
    Neg(Rc<Symbolic<F>>),
}

impl<B: Builder> Expression<B> for Symbolic<B::F> {
    type Value = Felt<B::F>;

    fn assign(&self, dst: Felt<B::F>, builder: &mut B) {
        match self {
            Symbolic::Const(c) => {
                dst.imm(*c, builder);
            }
            Symbolic::Value(v) => {
                v.assign(dst, builder);
            }
            Symbolic::Add(lhs, rhs) => match (&**lhs, &**rhs) {
                (Symbolic::Const(lhs), Symbolic::Const(rhs)) => {
                    let sum = *lhs + *rhs;
                    builder.push(AsmInstruction::IMM(dst.0, sum));
                }
                (Symbolic::Const(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::ADDI(dst.0, rhs.0, *lhs));
                }
                (Symbolic::Const(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::ADDI(dst.0, rhs_value.0, *lhs));
                }
                (Symbolic::Value(lhs), Symbolic::Const(rhs)) => {
                    builder.push(AsmInstruction::ADDI(dst.0, lhs.0, *rhs));
                }
                (Symbolic::Value(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::ADD(dst.0, lhs.0, rhs.0));
                }
                (Symbolic::Value(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::ADD(dst.0, lhs.0, rhs_value.0));
                }
                (lhs, Symbolic::Const(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::ADDI(dst.0, lhs_value.0, *rhs));
                }
                (lhs, Symbolic::Value(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::ADD(dst.0, lhs_value.0, rhs.0));
                }
                (lhs, rhs) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::ADD(dst.0, lhs_value.0, rhs_value.0));
                }
            },
            Symbolic::Mul(lhs, rhs) => match (&**lhs, &**rhs) {
                (Symbolic::Const(lhs), Symbolic::Const(rhs)) => {
                    let product = *lhs * *rhs;
                    builder.push(AsmInstruction::IMM(dst.0, product));
                }
                (Symbolic::Const(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::MULI(dst.0, rhs.0, *lhs));
                }
                (Symbolic::Const(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::MULI(dst.0, rhs_value.0, *lhs));
                }
                (Symbolic::Value(lhs), Symbolic::Const(rhs)) => {
                    builder.push(AsmInstruction::MULI(dst.0, lhs.0, *rhs));
                }
                (Symbolic::Value(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::MUL(dst.0, lhs.0, rhs.0));
                }
                (Symbolic::Value(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::MUL(dst.0, lhs.0, rhs_value.0));
                }
                (lhs, Symbolic::Const(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::MULI(dst.0, lhs_value.0, *rhs));
                }
                (lhs, Symbolic::Value(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::MUL(dst.0, lhs_value.0, rhs.0));
                }
                (lhs, rhs) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::MUL(dst.0, lhs_value.0, rhs_value.0));
                }
            },
            Symbolic::Sub(lhs, rhs) => match (&**lhs, &**rhs) {
                (Symbolic::Const(lhs), Symbolic::Const(rhs)) => {
                    let difference = *lhs - *rhs;
                    builder.push(AsmInstruction::IMM(dst.0, difference));
                }
                (Symbolic::Const(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::SUBIN(dst.0, *lhs, rhs.0));
                }
                (Symbolic::Const(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::SUBIN(dst.0, *lhs, rhs_value.0));
                }
                (Symbolic::Value(lhs), Symbolic::Const(rhs)) => {
                    builder.push(AsmInstruction::SUBI(dst.0, lhs.0, *rhs));
                }
                (Symbolic::Value(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::SUB(dst.0, lhs.0, rhs.0));
                }
                (Symbolic::Value(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::SUB(dst.0, lhs.0, rhs_value.0));
                }
                (lhs, Symbolic::Const(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::SUBI(dst.0, lhs_value.0, *rhs));
                }
                (lhs, Symbolic::Value(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::SUB(dst.0, lhs_value.0, rhs.0));
                }
                (lhs, rhs) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::SUB(dst.0, lhs_value.0, rhs_value.0));
                }
            },
            Symbolic::Div(lhs, rhs) => match (&**lhs, &**rhs) {
                (Symbolic::Const(lhs), Symbolic::Const(rhs)) => {
                    let quotient = *lhs / *rhs;
                    builder.push(AsmInstruction::IMM(dst.0, quotient));
                }
                (Symbolic::Const(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::DIVIN(dst.0, *lhs, rhs.0));
                }
                (Symbolic::Const(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::DIVIN(dst.0, *lhs, rhs_value.0));
                }
                (Symbolic::Value(lhs), Symbolic::Const(rhs)) => {
                    builder.push(AsmInstruction::DIVI(dst.0, lhs.0, *rhs));
                }
                (Symbolic::Value(lhs), Symbolic::Value(rhs)) => {
                    builder.push(AsmInstruction::DIV(dst.0, lhs.0, rhs.0));
                }
                (Symbolic::Value(lhs), rhs) => {
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::DIV(dst.0, lhs.0, rhs_value.0));
                }
                (lhs, Symbolic::Const(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::DIVI(dst.0, lhs_value.0, *rhs));
                }
                (lhs, Symbolic::Value(rhs)) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    builder.push(AsmInstruction::DIV(dst.0, lhs_value.0, rhs.0));
                }
                (lhs, rhs) => {
                    let lhs_value = Felt::uninit(builder);
                    lhs.assign(lhs_value, builder);
                    let rhs_value = Felt::uninit(builder);
                    rhs.assign(rhs_value, builder);
                    builder.push(AsmInstruction::DIV(dst.0, lhs_value.0, rhs_value.0));
                }
            },
            Symbolic::Neg(operand) => match &**operand {
                Symbolic::Const(operand) => {
                    let negated = -*operand;
                    builder.push(AsmInstruction::IMM(dst.0, negated));
                }
                Symbolic::Value(operand) => {
                    builder.push(AsmInstruction::SUBIN(dst.0, B::F::zero(), operand.0));
                }
                operand => {
                    let operand_value = Felt::uninit(builder);
                    operand.assign(operand_value, builder);
                    builder.push(AsmInstruction::SUBIN(dst.0, B::F::zero(), operand_value.0));
                }
            },
        }
    }
}

impl<F> From<Felt<F>> for Symbolic<F> {
    fn from(value: Felt<F>) -> Self {
        Symbolic::Value(value)
    }
}

impl<F> Add for Symbolic<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Symbolic::Add(Rc::new(self), Rc::new(rhs))
    }
}

impl<F> Mul for Symbolic<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Symbolic::Mul(Rc::new(self), Rc::new(rhs))
    }
}

impl<F> Sub for Symbolic<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Symbolic::Sub(Rc::new(self), Rc::new(rhs))
    }
}

impl<F> Div for Symbolic<F> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Symbolic::Div(Rc::new(self), Rc::new(rhs))
    }
}

impl<F> Neg for Symbolic<F> {
    type Output = Self;

    fn neg(self) -> Self {
        Symbolic::Neg(Rc::new(self))
    }
}

impl<F> Add<Felt<F>> for Symbolic<F> {
    type Output = Self;

    fn add(self, rhs: Felt<F>) -> Self {
        Symbolic::Add(Rc::new(self), Rc::new(Symbolic::Value(rhs)))
    }
}

impl<F> Add<F> for Symbolic<F> {
    type Output = Self;

    fn add(self, rhs: F) -> Self {
        Symbolic::Add(Rc::new(self), Rc::new(Symbolic::Const(rhs)))
    }
}

impl<F> Mul<Felt<F>> for Symbolic<F> {
    type Output = Self;

    fn mul(self, rhs: Felt<F>) -> Self {
        Symbolic::Mul(Rc::new(self), Rc::new(Symbolic::Value(rhs)))
    }
}

impl<F> Mul<F> for Symbolic<F> {
    type Output = Self;

    fn mul(self, rhs: F) -> Self {
        Symbolic::Mul(Rc::new(self), Rc::new(Symbolic::Const(rhs)))
    }
}

impl<F> Sub<Felt<F>> for Symbolic<F> {
    type Output = Self;

    fn sub(self, rhs: Felt<F>) -> Self {
        Symbolic::Sub(Rc::new(self), Rc::new(Symbolic::Value(rhs)))
    }
}

impl<F> Sub<F> for Symbolic<F> {
    type Output = Self;

    fn sub(self, rhs: F) -> Self {
        Symbolic::Sub(Rc::new(self), Rc::new(Symbolic::Const(rhs)))
    }
}

impl<F> Div<Felt<F>> for Symbolic<F> {
    type Output = Self;

    fn div(self, rhs: Felt<F>) -> Self {
        Symbolic::Div(Rc::new(self), Rc::new(Symbolic::Value(rhs)))
    }
}

impl<F> Div<F> for Symbolic<F> {
    type Output = Self;

    fn div(self, rhs: F) -> Self {
        Symbolic::Div(Rc::new(self), Rc::new(Symbolic::Const(rhs)))
    }
}
