use crate::asm::AsmInstruction;
use crate::ir::Constant;
use crate::ir::Variable;

use crate::ir::Expression;
use crate::ir::Felt;

use p3_field::AbstractField;
use p3_field::PrimeField32;

pub trait Builder: Sized {
    type F: PrimeField32;
    /// Get stack memory.
    fn get_mem(&mut self, size: usize) -> i32;
    //  Allocate heap memory.
    // fn alloc(&mut self, size: Int) -> Int;

    fn push(&mut self, instruction: AsmInstruction<Self::F>);

    fn push_to_block(&mut self, block_label: Self::F, instruction: AsmInstruction<Self::F>);

    fn basic_block(&mut self);

    fn block_label(&mut self) -> Self::F;

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

    // fn if(&mut self, condition: )
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

    fn push_to_block(&mut self, block_label: Self::F, instruction: AsmInstruction<Self::F>) {
        self.builder.push_to_block(block_label, instruction);
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
