use criterion::{black_box, criterion_group, criterion_main, Criterion};
use succinct_core::{
    runtime::{Program, Runtime},
    utils::prove,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    #[cfg(not(feature = "perf"))]
    unreachable!("--features=perf must be enabled to run this benchmark");

    let programs = ["../programs/sha2", "../programs/ssz_withdrawals"];
    for p in programs {
        let program = Program::from_elf(p);
        let cycles = {
            let mut runtime = Runtime::new(program.clone());
            runtime.run();
            runtime.global_clk
        };
        println!("program={} cycles={}", p, cycles);
        c.bench_function(p, |b| b.iter(|| prove(black_box(program.clone()))));
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
