#[cfg(test)]
pub mod tests {
    pub const CYCLE_TRACKER_ELF: &[u8] =
        include_bytes!("../../../examples/cycle-tracker/elf/riscv32im-succinct-zkvm-elf");

    pub const ED_ADD_ELF: &[u8] =
        include_bytes!("../../../examples/ed-add/elf/riscv32im-succinct-zkvm-elf");

    pub const ED_DECOMPRESS_ELF: &[u8] =
        include_bytes!("../../../examples/ed-decompress/elf/riscv32im-succinct-zkvm-elf");

    pub const ED25519_ELF: &[u8] =
        include_bytes!("../../../examples/ed25519/elf/riscv32im-succinct-zkvm-elf");

    pub const FIBONACCI_ELF: &[u8] =
        include_bytes!("../../../examples/fibonacci/elf/riscv32im-succinct-zkvm-elf");

    pub const IO_ELF: &[u8] =
        include_bytes!("../../../examples/io/elf/riscv32im-succinct-zkvm-elf");

    pub const KECCAK_PERMUTE_ELF: &[u8] =
        include_bytes!("../../../examples/keccak-permute/elf/riscv32im-succinct-zkvm-elf");

    pub const SECP256K1_ADD_ELF: &[u8] =
        include_bytes!("../../../examples/secp256k1-add/elf/riscv32im-succinct-zkvm-elf");

    pub const SECP256K1_DECOMPRESS_ELF: &[u8] =
        include_bytes!("../../../examples/secp256k1-decompress/elf/riscv32im-succinct-zkvm-elf");

    pub const SECP256K1_DOUBLE_ELF: &[u8] =
        include_bytes!("../../../examples/secp256k1-double/elf/riscv32im-succinct-zkvm-elf");

    pub const SHA_COMPRESS_ELF: &[u8] =
        include_bytes!("../../../examples/sha-compress/elf/riscv32im-succinct-zkvm-elf");

    pub const SHA_EXTEND_ELF: &[u8] =
        include_bytes!("../../../examples/sha-extend/elf/riscv32im-succinct-zkvm-elf");

    pub const SHA2_ELF: &[u8] =
        include_bytes!("../../../examples/sha2/elf/riscv32im-succinct-zkvm-elf");

    pub const SSZ_WITHDRAWALS_ELF: &[u8] =
        include_bytes!("../../../examples/ssz-withdrawals/elf/riscv32im-succinct-zkvm-elf");

    pub const TENDERMINT_ELF: &[u8] =
        include_bytes!("../../../examples/tendermint/elf/riscv32im-succinct-zkvm-elf");
}
