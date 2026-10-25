pub mod frame;
pub mod opcode;
pub mod value;
pub mod vm;

pub use frame::CallFrame;
pub use opcode::{DalvikInstruction, InstructionDecoder};
pub use value::Value;
pub use vm::{DexVm, ExecutionResult, NativeMethodHandler};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_vm_arithmetic() {
        let vm = DexVm::new();

        // Dalvik bytecode for:
        // const/4 v0, #5       -> 0x12, nibble_a=0, nibble_b=5 -> 0x5012
        // const/4 v1, #7       -> 0x12, nibble_a=1, nibble_b=7 -> 0x7112
        // add-int v2, v0, v1   -> 0x90, dst=2, src1=0, src2=1 -> 0x0290, 0x0100
        // return v2            -> 0x0f, src=2 -> 0x020f
        let insns = vec![
            0x5012,
            0x7112,
            0x0290,
            0x0100,
            0x020f,
        ];

        let result = vm.execute_bytecode(&insns, 3, 0, &[]);
        assert_eq!(result, ExecutionResult::Returned(Some(Value::Int(12))));
    }

    #[test]
    fn test_dex_vm_branching() {
        let vm = DexVm::new();

        // Dalvik bytecode for:
        // const/4 v0, #10      -> 0xa012
        // const/4 v1, #20      -> 0x2112 (b=2, not 20, let's use const/16)
        // const/16 v1, 20      -> 0x0113, 0x0014
        // if-lt v0, v1, +4     -> 0x34 (v0=0, v1=1, offset=4) -> 0x1034, 0x0004
        // const/4 v2, #1       -> 0x1212
        // goto +3              -> 0x28 (offset=3) -> 0x0328
        // const/4 v2, #99      -> (at pc=5): const/16 v2, 99 -> 0x0213, 0x0063
        // return v2            -> 0x020f
        let insns = vec![
            0xa012,         // pc=0: const/4 v0, #10
            0x0113, 0x0014, // pc=1: const/16 v1, #20
            0x1034, 0x0004, // pc=3: if-lt v0, v1, +4 -> targets pc = 3 + 4 = 7
            0x1212,         // pc=5: const/4 v2, #1
            0x0328,         // pc=6: goto +3 -> pc = 6 + 3 = 9
            0x0213, 0x0063, // pc=7: const/16 v2, #99
            0x020f,         // pc=9: return v2
        ];

        let result = vm.execute_bytecode(&insns, 3, 0, &[]);
        assert_eq!(result, ExecutionResult::Returned(Some(Value::Int(99))));
    }
}
