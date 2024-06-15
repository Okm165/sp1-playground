use std::{array, borrow::Borrow, ops::Deref};

use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::Matrix;
use sp1_core::air::BaseAirBuilder;
use sp1_primitives::RC_16_30_U32;

use crate::{air::SP1RecursionAirBuilder, memory::MemoryCols, runtime::Opcode};

pub mod control_flow;

use super::{
    columns::{
        control_flow::ControlFlow, memory::Memory, opcode_workspace::OpcodeWorkspace,
        permutation::Permutation, syscall_params::SyscallParams, Poseidon2, Poseidon2Degree3,
        Poseidon2Degree9, NUM_POSEIDON2_DEGREE3_COLS, NUM_POSEIDON2_DEGREE9_COLS,
    },
    external_linear_layer, internal_linear_layer, Poseidon2WideChip, NUM_EXTERNAL_ROUNDS,
    NUM_INTERNAL_ROUNDS, WIDTH,
};

impl<F, const DEGREE: usize> BaseAir<F> for Poseidon2WideChip<DEGREE> {
    fn width(&self) -> usize {
        if DEGREE == 3 {
            NUM_POSEIDON2_DEGREE3_COLS
        } else if DEGREE == 9 {
            NUM_POSEIDON2_DEGREE9_COLS
        } else {
            panic!("Unsupported degree: {}", DEGREE);
        }
    }
}

impl<AB, const DEGREE: usize> Air<AB> for Poseidon2WideChip<DEGREE>
where
    AB: SP1RecursionAirBuilder,
    AB::Var: 'static,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local_row = Self::convert::<AB>(main.row_slice(0));
        let next_row = Self::convert::<AB>(main.row_slice(1));

        // Check that all the control flow columns are correct.
        let local_control_flow = local_row.control_flow();
        let next_control_flow = next_row.control_flow();
        self.eval_control_flow(builder, local_row.as_ref(), next_row.as_ref());

        // // Check that the syscall columns are correct.
        let local_syscall = local_row.syscall_params();
        let next_syscall = next_row.syscall_params();
        self.eval_syscall_params(
            builder,
            local_syscall,
            next_syscall,
            local_control_flow,
            next_control_flow,
        );

        // Check that all the memory access columns are correct.
        let local_opcode_workspace = local_row.opcode_workspace();
        self.eval_mem(
            builder,
            local_syscall,
            local_row.memory(),
            local_opcode_workspace,
            local_control_flow,
        );

        // Check that the permutation columns are correct.
        let local_perm_cols = local_row.permutation();
        self.eval_perm(
            builder,
            local_perm_cols.as_ref(),
            local_row.memory(),
            local_row.opcode_workspace(),
            local_control_flow,
        );

        // Check that the permutation output is copied to the next row correctly.
        self.eval_row_transition(
            builder,
            local_row.control_flow(),
            local_row.opcode_workspace(),
            next_row.opcode_workspace(),
            local_row.permutation().as_ref(),
            local_row.memory(),
            next_row.memory(),
        );
    }
}

impl<'a, const DEGREE: usize> Poseidon2WideChip<DEGREE> {
    fn convert<AB: SP1RecursionAirBuilder>(
        row: impl Deref<Target = [AB::Var]>,
    ) -> Box<dyn Poseidon2<'a, AB::Var> + 'a>
    where
        AB::Var: 'a,
    {
        if DEGREE == 3 {
            let convert: &Poseidon2Degree3<AB::Var> = (*row).borrow();
            Box::new(*convert)
        } else if DEGREE == 9 {
            let convert: &Poseidon2Degree9<AB::Var> = (*row).borrow();
            Box::new(*convert)
        } else {
            panic!("Unsupported degree");
        }
    }

    fn eval_syscall_params<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        local_syscall: &SyscallParams<AB::Var>,
        next_syscall: &SyscallParams<AB::Var>,
        local_control_flow: &ControlFlow<AB::Var>,
        next_control_flow: &ControlFlow<AB::Var>,
    ) {
        // Constraint that the operands are sent from the CPU table.
        let operands = local_syscall.get_raw_params();
        let opcodes: [AB::Expr; 3] = [
            Opcode::Poseidon2Compress,
            Opcode::Poseidon2Absorb,
            Opcode::Poseidon2Finalize,
        ]
        .map(|x| x.as_field::<AB::F>().into());
        let opcode_selectors = [
            local_control_flow.is_compress,
            local_control_flow.is_absorb,
            local_control_flow.is_finalize,
        ];

        let opcode: AB::Expr = opcodes
            .iter()
            .zip(opcode_selectors.iter())
            .map(|(x, y)| x.clone() * *y)
            .sum();

        builder.receive_table(opcode, &operands, local_control_flow.is_syscall);

        let mut transition_builder = builder.when_transition();

        // Apply syscall constraints for compress.  Verify that the syscall parameters are copied to
        // the compress output row.
        {
            let mut compress_syscall_builder = transition_builder
                .when(local_control_flow.is_compress * local_control_flow.is_syscall);

            let local_syscall_params = local_syscall.compress();
            let next_syscall_params = next_syscall.compress();
            compress_syscall_builder.assert_eq(local_syscall_params.clk, next_syscall_params.clk);
            compress_syscall_builder
                .assert_eq(local_syscall_params.dst_ptr, next_syscall_params.dst_ptr);
            compress_syscall_builder
                .assert_eq(local_syscall_params.left_ptr, next_syscall_params.left_ptr);
            compress_syscall_builder.assert_eq(
                local_syscall_params.right_ptr,
                next_syscall_params.right_ptr,
            );
        }

        // Apply syscall constraints for absorb.  Verify that the syscall parameters are the same within
        // an absorb call.
        {
            let mut absorb_syscall_builder = transition_builder.when(local_control_flow.is_absorb);
            let mut absorb_syscall_builder =
                absorb_syscall_builder.when_not(next_control_flow.is_syscall);

            let local_syscall_params = local_syscall.absorb();
            let next_syscall_params = next_syscall.absorb();

            absorb_syscall_builder.assert_eq(local_syscall_params.clk, next_syscall_params.clk);
            absorb_syscall_builder
                .assert_eq(local_syscall_params.hash_num, next_syscall_params.hash_num);
            absorb_syscall_builder.assert_eq(
                local_syscall_params.input_ptr,
                next_syscall_params.input_ptr,
            );
            absorb_syscall_builder.assert_eq(local_syscall_params.len, next_syscall_params.len);
        }
    }

    fn eval_mem<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        syscall_params: &SyscallParams<AB::Var>,
        memory: &Memory<AB::Var>,
        opcode_workspace: &OpcodeWorkspace<AB::Var>,
        control_flow: &ControlFlow<AB::Var>,
    ) {
        let clk = syscall_params.get_raw_params()[0];
        let is_real = control_flow.is_compress + control_flow.is_absorb + control_flow.is_finalize;

        // Verify the memory flags.
        for i in 0..WIDTH / 2 {
            builder.assert_bool(memory.memory_slot_used[i]);
            builder
                .when(memory.memory_slot_used[i])
                .assert_one(is_real.clone());

            // For compress and finalize, all of the slots should be true.
            builder
                .when(control_flow.is_compress + control_flow.is_finalize)
                .assert_one(memory.memory_slot_used[i]);

            // For absorb, the index of the first non zero slot should be equal to the state_cursor.
            // The number of sequential non zero slots should be equal to the number of consumed elements.
            // Need to make sure the non zero slots are contiguous.
            // TODO
        }

        // Verify the memory addr.
        builder
            .when(control_flow.is_compress * control_flow.is_syscall)
            .assert_eq(syscall_params.compress().left_ptr, memory.start_addr);
        builder
            .when(control_flow.is_compress_output)
            .assert_eq(syscall_params.compress().dst_ptr, memory.start_addr);
        builder
            .when(control_flow.is_absorb * control_flow.is_syscall)
            .assert_eq(syscall_params.absorb().input_ptr, memory.start_addr);
        // TODO: Need to handle the case for non syscall compress.
        builder
            .when(control_flow.is_finalize)
            .assert_eq(syscall_params.finalize().output_ptr, memory.start_addr);

        // Evaluate the first half of the memory.
        let mut addr: AB::Expr = memory.start_addr.into();
        for i in 0..WIDTH / 2 {
            builder.recursion_eval_memory_access_single(
                clk + control_flow.is_compress_output,
                addr.clone(),
                &memory.memory_accesses[i],
                memory.memory_slot_used[i],
            );

            // For read only accesses, assert the value didn't change.
            builder
                .when(control_flow.is_compress * control_flow.is_syscall + control_flow.is_absorb)
                .assert_eq(
                    *memory.memory_accesses[i].prev_value(),
                    *memory.memory_accesses[i].value(),
                );

            addr = addr.clone() + memory.memory_slot_used[i].into();
        }

        // Evalulate the second half for compress syscall.
        let compress_workspace = opcode_workspace.compress();
        // Verify the start addr.
        builder
            .when(control_flow.is_compress * control_flow.is_syscall)
            .assert_eq(
                compress_workspace.start_addr,
                syscall_params.compress().right_ptr,
            );
        builder.when(control_flow.is_compress_output).assert_eq(
            compress_workspace.start_addr,
            syscall_params.compress().dst_ptr + AB::Expr::from_canonical_usize(WIDTH / 2),
        );
        // Evaluate then memory
        let mut addr: AB::Expr = compress_workspace.start_addr.into();
        for i in 0..WIDTH / 2 {
            builder.recursion_eval_memory_access_single(
                clk + control_flow.is_compress_output,
                addr.clone(),
                &compress_workspace.memory_accesses[i],
                control_flow.is_compress,
            );

            builder
                .when(control_flow.is_syscall * control_flow.is_compress)
                .assert_eq(
                    *compress_workspace.memory_accesses[i].prev_value(),
                    *compress_workspace.memory_accesses[i].value(),
                );

            addr = addr.clone() + AB::Expr::one();
        }
    }

    fn eval_perm<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        perm_cols: &dyn Permutation<AB::Var>,
        memory: &Memory<AB::Var>,
        opcode_workspace: &OpcodeWorkspace<AB::Var>,
        control_flow: &ControlFlow<AB::Var>,
    ) {
        let input: [AB::Expr; WIDTH] = array::from_fn(|i| {
            let previous_state = opcode_workspace.hash().previous_state[i];

            let (compress_input, absorb_input, finalize_input) = if i < WIDTH / 2 {
                let mem_value = *memory.memory_accesses[i].value();

                let compress_input = mem_value;
                let absorb_input =
                    builder.if_else(memory.memory_slot_used[i], mem_value, previous_state);
                let finalize_input = previous_state.into();

                (compress_input, absorb_input, finalize_input)
            } else {
                let compress_input =
                    *opcode_workspace.compress().memory_accesses[i - WIDTH / 2].value();
                let absorb_input = previous_state.into();
                let finalize_input = previous_state.into();

                (compress_input, absorb_input, finalize_input)
            };

            control_flow.is_compress * compress_input
                + control_flow.is_absorb * absorb_input
                + control_flow.is_finalize * finalize_input
        });

        // Apply the initial round.
        let initial_round_output = {
            let mut initial_round_output = input;
            external_linear_layer(&mut initial_round_output);
            initial_round_output
        };
        let external_round_0_state: [AB::Expr; WIDTH] = core::array::from_fn(|i| {
            let state = perm_cols.external_rounds_state()[0];
            state[i].into()
        });

        builder.assert_all_eq(external_round_0_state.clone(), initial_round_output);

        // Apply the first half of external rounds.
        for r in 0..NUM_EXTERNAL_ROUNDS / 2 {
            self.eval_external_round(builder, perm_cols, r);
        }

        // Apply the internal rounds.
        self.eval_internal_rounds(builder, perm_cols);

        // Apply the second half of external rounds.
        for r in NUM_EXTERNAL_ROUNDS / 2..NUM_EXTERNAL_ROUNDS {
            self.eval_external_round(builder, perm_cols, r);
        }
    }

    fn eval_external_round<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        perm_cols: &dyn Permutation<AB::Var>,
        r: usize,
    ) {
        let external_state = perm_cols.external_rounds_state()[r];

        // Add the round constants.
        let round = if r < NUM_EXTERNAL_ROUNDS / 2 {
            r
        } else {
            r + NUM_INTERNAL_ROUNDS
        };
        let add_rc: [AB::Expr; WIDTH] = core::array::from_fn(|i| {
            external_state[i].into() + AB::F::from_wrapped_u32(RC_16_30_U32[round][i])
        });

        // Apply the sboxes.
        // See `populate_external_round` for why we don't have columns for the sbox output here.
        let mut sbox_deg_7: [AB::Expr; WIDTH] = core::array::from_fn(|_| AB::Expr::zero());
        let mut sbox_deg_3: [AB::Expr; WIDTH] = core::array::from_fn(|_| AB::Expr::zero());
        for i in 0..WIDTH {
            let calculated_sbox_deg_3 = add_rc[i].clone() * add_rc[i].clone() * add_rc[i].clone();

            if let Some(external_sbox) = perm_cols.external_rounds_sbox() {
                builder.assert_eq(external_sbox[r][i].into(), calculated_sbox_deg_3);
                sbox_deg_3[i] = external_sbox[r][i].into();
            } else {
                sbox_deg_3[i] = calculated_sbox_deg_3;
            }

            sbox_deg_7[i] = sbox_deg_3[i].clone() * sbox_deg_3[i].clone() * add_rc[i].clone();
        }

        // Apply the linear layer.
        let mut state = sbox_deg_7;
        external_linear_layer(&mut state);

        let next_state_cols = if r == NUM_EXTERNAL_ROUNDS / 2 - 1 {
            perm_cols.internal_rounds_state()
        } else if r == NUM_EXTERNAL_ROUNDS - 1 {
            perm_cols.perm_output()
        } else {
            &perm_cols.external_rounds_state()[r + 1]
        };
        for i in 0..WIDTH {
            builder.assert_eq(next_state_cols[i], state[i].clone());
        }
    }

    fn eval_internal_rounds<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        perm_cols: &dyn Permutation<AB::Var>,
    ) {
        let state = &perm_cols.internal_rounds_state();
        let s0 = perm_cols.internal_rounds_s0();
        let mut state: [AB::Expr; WIDTH] = core::array::from_fn(|i| state[i].into());
        for r in 0..NUM_INTERNAL_ROUNDS {
            // Add the round constant.
            let round = r + NUM_EXTERNAL_ROUNDS / 2;
            let add_rc = if r == 0 {
                state[0].clone()
            } else {
                s0[r - 1].into()
            } + AB::Expr::from_wrapped_u32(RC_16_30_U32[round][0]);

            let mut sbox_deg_3 = add_rc.clone() * add_rc.clone() * add_rc.clone();
            if let Some(internal_sbox) = perm_cols.internal_rounds_sbox() {
                builder.assert_eq(internal_sbox[r], sbox_deg_3);
                sbox_deg_3 = internal_sbox[r].into();
            }

            // See `populate_internal_rounds` for why we don't have columns for the sbox output here.
            let sbox_deg_7 = sbox_deg_3.clone() * sbox_deg_3.clone() * add_rc.clone();

            // Apply the linear layer.
            // See `populate_internal_rounds` for why we don't have columns for the new state here.
            state[0] = sbox_deg_7.clone();
            internal_linear_layer(&mut state);

            if r < NUM_INTERNAL_ROUNDS - 1 {
                builder.assert_eq(s0[r], state[0].clone());
            }
        }

        let external_state = perm_cols.external_rounds_state()[NUM_EXTERNAL_ROUNDS / 2];
        for i in 0..WIDTH {
            builder.assert_eq(external_state[i], state[i].clone())
        }
    }

    fn eval_row_transition<AB: SP1RecursionAirBuilder>(
        &self,
        builder: &mut AB,
        control_flow: &ControlFlow<AB::Var>,
        local_opcode_workspace: &OpcodeWorkspace<AB::Var>,
        next_opcode_workspace: &OpcodeWorkspace<AB::Var>,
        permutation: &dyn Permutation<AB::Var>,
        local_memory: &Memory<AB::Var>,
        next_memory: &Memory<AB::Var>,
    ) {
        // For compress syscall rows, contrain that the permutation's output is equal to the compress
        // output's memory values.
        {
            let next_memory_output: [AB::Var; WIDTH] = array::from_fn(|i| {
                if i < WIDTH / 2 {
                    *next_memory.memory_accesses[i].value()
                } else {
                    *next_opcode_workspace.compress().memory_accesses[i - WIDTH / 2].value()
                }
            });

            builder
                .when_transition()
                .when(control_flow.is_compress)
                .when(control_flow.is_syscall)
                .assert_all_eq(next_memory_output, *permutation.perm_output());
        }

        // Absorb
        {
            // TODO: check the do_perm flag.

            // Expected state when a permutation is done.
            builder
                .when(control_flow.is_absorb)
                .when(control_flow.do_perm)
                .assert_all_eq(
                    local_opcode_workspace.hash().state,
                    *permutation.perm_output(),
                );

            // TODO: move the permutation input as a method for the poseidon2 struct.
            let input: [AB::Expr; WIDTH] = array::from_fn(|i| {
                if i < WIDTH / 2 {
                    builder.if_else(
                        local_memory.memory_slot_used[i],
                        *local_memory.memory_accesses[i].value(),
                        local_opcode_workspace.hash().previous_state[i],
                    )
                } else {
                    local_opcode_workspace.hash().previous_state[i].into()
                }
            });

            builder
                .when(control_flow.is_absorb_no_perm)
                .assert_all_eq(local_opcode_workspace.hash().state, input);

            builder
                .when_transition()
                .when(control_flow.is_absorb)
                .assert_all_eq(
                    local_opcode_workspace.hash().state,
                    next_opcode_workspace.hash().previous_state,
                );
        }

        // Finalize
        {
            // TODO: check the do_perm flag.

            builder
                .when(control_flow.is_finalize)
                .when(control_flow.do_perm)
                .assert_all_eq(
                    local_opcode_workspace.hash().state,
                    *permutation.perm_output(),
                );

            builder
                .when(control_flow.is_finalize)
                .when_not(control_flow.do_perm)
                .assert_all_eq(
                    local_opcode_workspace.hash().state,
                    local_opcode_workspace.hash().previous_state,
                );
        }
    }
}
