use std::mem::{size_of, transmute};

use sp1_core::utils::indices_arr;
use sp1_derive::AlignedBorrow;

use self::{
    control_flow::ControlFlow,
    opcode_workspace::OpcodeWorkspace,
    permutation::{Permutation, PermutationNoSbox, PermutationSBox},
    syscall_params::SyscallParams,
};

use super::WIDTH;

pub mod control_flow;
pub mod opcode_workspace;
pub mod permutation;
pub mod syscall_params;

pub trait Poseidon2<'a, T: Copy + 'a> {
    fn control_flow(&self) -> &ControlFlow<T>;

    fn syscall_params(&self) -> &SyscallParams<T>;

    fn opcode_workspace(&self) -> &OpcodeWorkspace<T>;

    fn permutation(&self) -> Box<dyn Permutation<T> + 'a>;
}

pub trait Poseidon2Mut<'a, T: Copy + 'a> {
    fn control_flow_mut(&mut self) -> &mut ControlFlow<T>;

    fn syscall_params_mut(&mut self) -> &mut SyscallParams<T>;

    fn opcode_workspace_mut(&mut self) -> &mut OpcodeWorkspace<T>;
}

enum MyEnum<T: Copy> {
    P2Degree3(Poseidon2Degree3<T>),
    P2Degree8(Poseidon2Degree7<T>),
}

impl<'a, T: Copy + 'a> Poseidon2<'a, T> for MyEnum<T> {
    // type Perm = PermutationSBox<T>;

    fn control_flow(&self) -> &ControlFlow<T> {
        match self {
            MyEnum::P2Degree3(p) => p.control_flow(),
            MyEnum::P2Degree8(p) => p.control_flow(),
        }
    }

    fn syscall_params(&self) -> &SyscallParams<T> {
        match self {
            MyEnum::P2Degree3(p) => p.syscall_params(),
            MyEnum::P2Degree8(p) => p.syscall_params(),
        }
    }

    fn opcode_workspace(&self) -> &OpcodeWorkspace<T> {
        match self {
            MyEnum::P2Degree3(p) => p.opcode_workspace(),
            MyEnum::P2Degree8(p) => p.opcode_workspace(),
        }
    }

    fn permutation(&self) -> Box<dyn Permutation<T> + 'a> {
        match self {
            MyEnum::P2Degree3(p) => p.permutation(),
            MyEnum::P2Degree8(p) => p.permutation(),
        }
    }
}

enum MyEnumMut<'a, T: Copy> {
    P2Degree3(&'a mut Poseidon2Degree3<T>),
    P2Degree8(&'a mut Poseidon2Degree7<T>),
}

impl<'a, T: Copy + 'a> Poseidon2Mut<'a, T> for MyEnumMut<'a, T> {
    fn control_flow_mut(&mut self) -> &mut ControlFlow<T> {
        match self {
            MyEnumMut::P2Degree3(p) => p.control_flow_mut(),
            MyEnumMut::P2Degree8(p) => p.control_flow_mut(),
        }
    }

    fn syscall_params_mut(&mut self) -> &mut SyscallParams<T> {
        match self {
            MyEnumMut::P2Degree3(p) => p.syscall_params_mut(),
            MyEnumMut::P2Degree8(p) => p.syscall_params_mut(),
        }
    }

    fn opcode_workspace_mut(&mut self) -> &mut OpcodeWorkspace<T> {
        match self {
            MyEnumMut::P2Degree3(p) => p.opcode_workspace_mut(),
            MyEnumMut::P2Degree8(p) => p.opcode_workspace_mut(),
        }
    }
}

pub const NUM_POSEIDON2_DEGREE3_COLS: usize = size_of::<Poseidon2Degree3<u8>>();

const fn make_col_map_degree3() -> Poseidon2Degree3<usize> {
    let indices_arr = indices_arr::<NUM_POSEIDON2_DEGREE3_COLS>();
    unsafe {
        transmute::<[usize; NUM_POSEIDON2_DEGREE3_COLS], Poseidon2Degree3<usize>>(indices_arr)
    }
}
pub const POSEIDON2_DEGREE3_COL_MAP: Poseidon2Degree3<usize> = make_col_map_degree3();

#[derive(AlignedBorrow, Clone, Copy)]
#[repr(C)]
pub struct Poseidon2Degree3<T: Copy> {
    pub control_flow: ControlFlow<T>,
    pub syscall_input: SyscallParams<T>,
    pub opcode_specific_cols: OpcodeWorkspace<T>,
    pub permutation_cols: PermutationSBox<T>,
    pub state_cursor: [T; WIDTH / 2], // Only used for absorb
}

impl<'a, T: Copy + 'a> Poseidon2<'a, T> for Poseidon2Degree3<T> {
    fn control_flow(&self) -> &ControlFlow<T> {
        &self.control_flow
    }

    fn syscall_params(&self) -> &SyscallParams<T> {
        &self.syscall_input
    }

    fn opcode_workspace(&self) -> &OpcodeWorkspace<T> {
        &self.opcode_specific_cols
    }

    fn permutation(&self) -> Box<dyn Permutation<T> + 'a> {
        Box::new(self.permutation_cols)
    }
}

impl<'a, T: Copy + 'a> Poseidon2Mut<'a, T> for &'a mut Poseidon2Degree3<T> {
    fn control_flow_mut(&mut self) -> &mut ControlFlow<T> {
        &mut self.control_flow
    }

    fn syscall_params_mut(&mut self) -> &mut SyscallParams<T> {
        &mut self.syscall_input
    }

    fn opcode_workspace_mut(&mut self) -> &mut OpcodeWorkspace<T> {
        &mut self.opcode_specific_cols
    }
}

pub const NUM_POSEIDON2_DEGREE7_COLS: usize = size_of::<Poseidon2Degree7<u8>>();
const fn make_col_map_degree7() -> Poseidon2Degree7<usize> {
    let indices_arr = indices_arr::<NUM_POSEIDON2_DEGREE7_COLS>();
    unsafe {
        transmute::<[usize; NUM_POSEIDON2_DEGREE7_COLS], Poseidon2Degree7<usize>>(indices_arr)
    }
}
pub const POSEIDON2_DEGREE7_COL_MAP: Poseidon2Degree7<usize> = make_col_map_degree7();

#[derive(AlignedBorrow, Clone, Copy)]
#[repr(C)]
pub struct Poseidon2Degree7<T: Copy> {
    pub control_flow: ControlFlow<T>,
    pub syscall_input: SyscallParams<T>,
    pub opcode_specific_cols: OpcodeWorkspace<T>,
    pub permutation_cols: PermutationNoSbox<T>,
    pub state_cursor: [T; WIDTH / 2], // Only used for absorb
}

impl<'a, T: Copy + 'a> Poseidon2<'a, T> for Poseidon2Degree7<T> {
    fn control_flow(&self) -> &ControlFlow<T> {
        &self.control_flow
    }

    fn syscall_params(&self) -> &SyscallParams<T> {
        &self.syscall_input
    }

    fn opcode_workspace(&self) -> &OpcodeWorkspace<T> {
        &self.opcode_specific_cols
    }

    fn permutation(&self) -> Box<dyn Permutation<T> + 'a> {
        Box::new(self.permutation_cols)
    }
}

impl<'a, T: Copy + 'a> Poseidon2Mut<'a, T> for &'a mut Poseidon2Degree7<T> {
    fn control_flow_mut(&mut self) -> &mut ControlFlow<T> {
        &mut self.control_flow
    }

    fn syscall_params_mut(&mut self) -> &mut SyscallParams<T> {
        &mut self.syscall_input
    }

    fn opcode_workspace_mut(&mut self) -> &mut OpcodeWorkspace<T> {
        &mut self.opcode_specific_cols
    }
}
