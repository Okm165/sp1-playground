use crate::{
    runtime::{Register, Syscall, SyscallContext},
    utils::u32_to_comma_separated,
};

pub struct SyscallWrite;

impl SyscallWrite {
    pub fn new() -> Self {
        Self
    }
}

impl Syscall for SyscallWrite {
    fn execute(&self, ctx: &mut SyscallContext) -> u32 {
        let a0 = Register::X10;
        let a1 = Register::X11;
        let a2 = Register::X12;
        let rt = &mut ctx.rt;
        let fd = rt.register(a0);
        if fd == 1 || fd == 2 || fd == 3 || fd == 4 {
            let write_buf = rt.register(a1);
            let nbytes = rt.register(a2);
            // Read nbytes from memory starting at write_buf.
            let bytes = (0..nbytes)
                .map(|i| rt.byte(write_buf + i))
                .collect::<Vec<u8>>();
            let slice = bytes.as_slice();
            if fd == 1 {
                let s = core::str::from_utf8(slice).unwrap();
                if s.contains("cycle-tracker-start:") {
                    let fn_name = s
                        .split("cycle-tracker-start:")
                        .last()
                        .unwrap()
                        .trim_end()
                        .trim_start();
                    let depth = rt.cycle_tracker.len() as u32;
                    rt.cycle_tracker
                        .insert(fn_name.to_string(), (rt.state.global_clk, depth));
                    let padding = (0..depth).map(|_| "│ ").collect::<String>();
                    log::info!("{}┌╴{}", padding, fn_name);
                } else if s.contains("cycle-tracker-end:") {
                    let fn_name = s
                        .split("cycle-tracker-end:")
                        .last()
                        .unwrap()
                        .trim_end()
                        .trim_start();
                    let (start, depth) = rt.cycle_tracker.remove(fn_name).unwrap_or((0, 0));
                    // Leftpad by 2 spaces for each depth.
                    let padding = (0..depth).map(|_| "│ ").collect::<String>();
                    log::info!(
                        "{}└╴{} cycles",
                        padding,
                        u32_to_comma_separated(rt.state.global_clk - start)
                    );
                } else {
                    let flush_s = update_io_buf(ctx, fd, s);
                    if let Some(s) = flush_s {
                        log::info!("stdout: {}", s);
                    }
                }
            } else if fd == 2 {
                let s = core::str::from_utf8(slice).unwrap();
                let flush_s = update_io_buf(ctx, fd, s);
                if let Some(s) = flush_s {
                    log::info!("stderr: {}", s);
                }
            } else if fd == 3 {
                rt.state.output_stream.extend_from_slice(slice);
            } else if fd == 4 {
                rt.state.input_stream.extend_from_slice(slice);
            } else {
                unreachable!()
            }
        }
        0
    }
}

pub fn update_io_buf(ctx: &mut SyscallContext, fd: u32, s: &str) -> Option<String> {
    let rt = &mut ctx.rt;
    if s.ends_with('\n') {
        if let Some(existing) = rt.io_buf.remove(&fd) {
            Some(format!("{}{}", existing, s.trim_end()))
        } else {
            Some(format!("{}", s.trim_end()))
        }
    } else {
        ctx.rt
            .io_buf
            .entry(fd)
            .or_insert_with(String::new)
            .push_str(s);
        None
    }
}
