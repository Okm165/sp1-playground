use crate::{
    events::{Poseidon2PermuteEvent, PrecompileEvent},
    syscalls::{Syscall, SyscallCode, SyscallContext},
};

pub(crate) struct Poseidon2PermuteSyscall;

impl Syscall for Poseidon2PermuteSyscall {
    fn num_extra_cycles(&self) -> u32 {
        1
    }

    fn execute(
        &self,
        rt: &mut SyscallContext,
        syscall_code: SyscallCode,
        arg1: u32,
        arg2: u32,
    ) -> Option<u32> {
        let clk_init = rt.clk;
        let input_ptr = arg1;
        assert!(arg2 == 0, "arg2 must be 0");

        let input_ptr_init = input_ptr;

        let state = Vec::<u32>::new();

        let input_memory_records = rt.mw_slice(input_ptr, &state);

        // Push the SHA extend event.
        let lookup_id = rt.syscall_lookup_id;
        let shard = rt.current_shard();
        let event = PrecompileEvent::Poseidon2Permute(Poseidon2PermuteEvent {
            lookup_id,
            shard,
            clk: clk_init,
            input_ptr: input_ptr_init,
            input_memory_records,
            local_mem_access: rt.postprocess(),
        });
        let syscall_event =
            rt.rt.syscall_event(clk_init, syscall_code.syscall_id(), arg1, arg2, lookup_id);
        rt.add_precompile_event(syscall_code, syscall_event, event);

        None
    }
}
