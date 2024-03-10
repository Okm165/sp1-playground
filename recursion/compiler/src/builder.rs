use crate::asm::AsmInstruction;
use crate::ir::Constant;
use crate::ir::Variable;

use crate::asm::BasicBlock;
use crate::ir::Expression;
use crate::ir::Felt;
use crate::prelude::Symbolic;

use p3_field::AbstractField;
use p3_field::PrimeField32;

pub trait Builder: Sized {
    type F: PrimeField32;
    /// Get stack memory.
    fn get_mem(&mut self, size: usize) -> i32;
    //  Allocate heap memory.
    // fn alloc(&mut self, size: Int) -> Int;

    fn push(&mut self, instruction: AsmInstruction<Self::F>);

    fn get_block_mut(&mut self, label: Self::F) -> &mut BasicBlock<Self::F>;

    fn basic_block(&mut self);

    fn block_label(&mut self) -> Self::F;

    fn push_to_block(&mut self, block_label: Self::F, instruction: AsmInstruction<Self::F>) {
        self.get_block_mut(block_label).push(instruction);
    }

    fn uninit<T: Variable<Self>>(&mut self) -> T {
        T::uninit(self)
    }

    fn constant<T: Constant<Self>>(&mut self, value: T::Constant) -> T {
        let var = T::uninit(self);
        var.imm(value, self);
        var
    }

    fn assign<E: Expression<Self>>(&mut self, dst: E::Value, expr: E) {
        expr.assign(dst, self);
    }

    fn eval<E: Expression<Self>>(&mut self, expr: E) -> E::Value {
        let dst = E::Value::uninit(self);
        expr.assign(dst, self);
        dst
    }

    fn range(&mut self, start: Felt<Self::F>, end: Felt<Self::F>) -> ForBuilder<Self> {
        let loop_var = Felt::uninit(self);
        ForBuilder {
            builder: self,
            start,
            end,
            loop_var,
        }
    }

    fn if_eq<E1, E2>(&mut self, lhs: E1, rhs: E2) -> IfBuilder<Self>
    where
        E1: Into<Symbolic<Self::F>>,
        E2: Into<Symbolic<Self::F>>,
    {
        IfBuilder {
            builder: self,
            lhs: lhs.into(),
            rhs: rhs.into(),
            is_eq: true,
        }
    }
}

pub struct IfBuilder<'a, B: Builder> {
    builder: &'a mut B,
    lhs: Symbolic<B::F>,
    rhs: Symbolic<B::F>,
    is_eq: bool,
}

impl<'a, B: Builder> Builder for IfBuilder<'a, B> {
    type F = B::F;
    fn get_mem(&mut self, size: usize) -> i32 {
        self.builder.get_mem(size)
    }

    fn push(&mut self, instruction: AsmInstruction<B::F>) {
        self.builder.push(instruction);
    }

    fn get_block_mut(&mut self, label: Self::F) -> &mut BasicBlock<Self::F> {
        self.builder.get_block_mut(label)
    }

    fn basic_block(&mut self) {
        self.builder.basic_block();
    }

    fn block_label(&mut self) -> B::F {
        self.builder.block_label()
    }
}

impl<'a, B: Builder> IfBuilder<'a, B> {
    pub fn then<Func>(self, f: Func)
    where
        Func: FnOnce(&mut B),
    {
        let Self {
            builder,
            lhs,
            rhs,
            is_eq,
        } = self;
        // Get the label for the block after the if block, and generate the conditional branch
        // instruction to it, if the condition is not met.
        let after_if_block = builder.block_label() + B::F::two();
        match (lhs, rhs, is_eq) {
            (Symbolic::Const(lhs), Symbolic::Const(rhs), true) => {
                if lhs == rhs {
                    f(builder);
                    return;
                }
            }
            (Symbolic::Const(lhs), Symbolic::Const(rhs), false) => {
                if lhs != rhs {
                    f(builder);
                    return;
                }
            }
            (Symbolic::Const(lhs), Symbolic::Value(rhs), true) => {
                let instr = AsmInstruction::BNEI(after_if_block, rhs.0, lhs);
                builder.push(instr);
            }
            (Symbolic::Const(lhs), Symbolic::Value(rhs), false) => {
                let instr = AsmInstruction::BEQI(after_if_block, rhs.0, lhs);
                builder.push(instr);
            }
            (Symbolic::Const(lhs), rhs, true) => {
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BNEI(after_if_block, rhs.0, lhs);
                builder.push(instr);
            }
            (Symbolic::Const(lhs), rhs, false) => {
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BEQI(after_if_block, rhs.0, lhs);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), Symbolic::Const(rhs), true) => {
                let instr = AsmInstruction::BNEI(after_if_block, lhs.0, rhs);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), Symbolic::Const(rhs), false) => {
                let instr = AsmInstruction::BEQI(after_if_block, lhs.0, rhs);
                builder.push(instr);
            }
            (lhs, Symbolic::Const(rhs), true) => {
                let lhs = builder.eval(lhs);
                let instr = AsmInstruction::BNEI(after_if_block, lhs.0, rhs);
                builder.push(instr);
            }
            (lhs, Symbolic::Const(rhs), false) => {
                let lhs = builder.eval(lhs);
                let instr = AsmInstruction::BEQI(after_if_block, lhs.0, rhs);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), Symbolic::Value(rhs), true) => {
                let instr = AsmInstruction::BNE(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), Symbolic::Value(rhs), false) => {
                let instr = AsmInstruction::BEQ(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), rhs, true) => {
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BNE(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (Symbolic::Value(lhs), rhs, false) => {
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BEQ(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (lhs, Symbolic::Value(rhs), true) => {
                let lhs = builder.eval(lhs);
                let instr = AsmInstruction::BNE(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (lhs, Symbolic::Value(rhs), false) => {
                let lhs = builder.eval(lhs);
                let instr = AsmInstruction::BEQ(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (lhs, rhs, true) => {
                let lhs = builder.eval(lhs);
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BNE(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
            (lhs, rhs, false) => {
                let lhs = builder.eval(lhs);
                let rhs = builder.eval(rhs);
                let instr = AsmInstruction::BEQ(after_if_block, lhs.0, rhs.0);
                builder.push(instr);
            }
        }
        // Generate the block for the then branch.
        builder.basic_block();
        f(builder);
        // Generate the block for returning to the main flow.
        builder.basic_block();
    }

    pub fn then_or_else<ThenFunc, ElseFunc>(self, then_f: ThenFunc, else_f: ElseFunc)
    where
        ThenFunc: FnOnce(&mut B),
        ElseFunc: FnOnce(&mut B),
    {
    }
}

/// A builder for a for loop.
///
/// Starting with end < start will lead to undefined behavior!
pub struct ForBuilder<'a, B: Builder> {
    builder: &'a mut B,
    start: Felt<B::F>,
    end: Felt<B::F>,
    loop_var: Felt<B::F>,
}

impl<'a, B: Builder> Builder for ForBuilder<'a, B> {
    type F = B::F;
    fn get_mem(&mut self, size: usize) -> i32 {
        self.builder.get_mem(size)
    }

    fn push(&mut self, instruction: AsmInstruction<B::F>) {
        self.builder.push(instruction);
    }

    fn get_block_mut(&mut self, label: Self::F) -> &mut BasicBlock<Self::F> {
        self.builder.get_block_mut(label)
    }

    fn basic_block(&mut self) {
        self.builder.basic_block();
    }

    fn block_label(&mut self) -> B::F {
        self.builder.block_label()
    }
}

impl<'a, B: Builder> ForBuilder<'a, B> {
    pub fn for_each<Func>(&mut self, f: Func)
    where
        Func: FnOnce(Felt<B::F>, &mut Self),
    {
        // The function block structure:
        // - Setting the loop range
        // - Executing the loop body and incrementing the loop variable
        // - the loop condition
        let loop_var = self.loop_var;
        // Set the loop variable to the start of the range.
        self.assign(loop_var, self.start);
        // Save the label of the for loop call
        let loop_call_label = self.block_label();
        // A basic block for the loop body
        self.basic_block();
        // Save the loop body label for the loop condition.
        let loop_label = self.block_label();
        // The loop body.
        f(loop_var, self);
        self.assign(loop_var, loop_var + B::F::one());
        // Add a basic block for the loop condition.
        self.basic_block();
        // Jump to loop body if the loop condition still holds.
        let instr = AsmInstruction::BNE(loop_label, loop_var.0, self.end.0);
        self.push(instr);
        // Add a jump instruction to the loop condition in the following block
        let label = self.block_label();
        let instr = AsmInstruction::j(label, self);
        self.push_to_block(loop_call_label, instr);
    }
}
