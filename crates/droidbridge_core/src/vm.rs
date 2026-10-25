use std::collections::HashMap;
use crate::frame::CallFrame;
use crate::opcode::{DalvikInstruction, InstructionDecoder};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    Returned(Option<Value>),
    StepLimitExceeded,
    VerifyError(String),
    DivideByZero,
    Halted,
}

pub type NativeMethodHandler = Box<dyn Fn(&[Value]) -> Option<Value> + Send + Sync>;

pub struct DexVm {
    pub max_steps: usize,
    pub native_methods: HashMap<String, NativeMethodHandler>,
}

impl Default for DexVm {
    fn default() -> Self {
        let mut vm = Self {
            max_steps: 100_000,
            native_methods: HashMap::new(),
        };
        vm.register_builtin_host_methods();
        vm
    }
}

impl DexVm {
    pub fn new() -> Self {
        Self::default()
    }

    fn register_builtin_host_methods(&mut self) {
        // android.util.Log.i(String tag, String msg)
        self.register_native_method("android/util/Log.i", |args| {
            println!("[Android Log.i] arg count: {}", args.len());
            Some(Value::Int(1))
        });

        // java.lang.Math.max(int a, int b)
        self.register_native_method("java/lang/Math.max", |args| {
            if args.len() >= 2 {
                let a = args[0].as_int();
                let b = args[1].as_int();
                Some(Value::Int(a.max(b)))
            } else {
                Some(Value::Int(0))
            }
        });
    }

    pub fn register_native_method<F>(&mut self, signature: &str, handler: F)
    where
        F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static,
    {
        self.native_methods.insert(signature.to_string(), Box::new(handler));
    }

    pub fn execute_bytecode(
        &self,
        insns: &[u16],
        registers_size: usize,
        ins_size: usize,
        args: &[Value],
    ) -> ExecutionResult {
        let mut frame = CallFrame::new(registers_size, ins_size, args);
        let mut steps = 0;
        let mut return_result_register = Value::Null;

        while frame.pc < insns.len() {
            steps += 1;
            if steps > self.max_steps {
                return ExecutionResult::StepLimitExceeded;
            }

            let (inst, unit_count) = match InstructionDecoder::decode_next(insns, frame.pc) {
                Some(res) => res,
                None => return ExecutionResult::VerifyError(format!("Failed to decode instruction at PC={}", frame.pc)),
            };

            let next_pc = frame.pc + unit_count;

            match inst {
                DalvikInstruction::Nop => {
                    frame.pc = next_pc;
                }
                DalvikInstruction::Move { dst, src } => {
                    let val = frame.get_reg(src);
                    frame.set_reg(dst, val);
                    frame.pc = next_pc;
                }
                DalvikInstruction::MoveResult { dst } => {
                    frame.set_reg(dst, return_result_register.clone());
                    frame.pc = next_pc;
                }
                DalvikInstruction::ReturnVoid => {
                    return ExecutionResult::Returned(None);
                }
                DalvikInstruction::Return { src }
                | DalvikInstruction::ReturnWide { src }
                | DalvikInstruction::ReturnObject { src } => {
                    let val = frame.get_reg(src);
                    return ExecutionResult::Returned(Some(val));
                }
                DalvikInstruction::Const4 { dst, val } => {
                    frame.set_reg(dst, Value::Int(val as i32));
                    frame.pc = next_pc;
                }
                DalvikInstruction::Const16 { dst, val } => {
                    frame.set_reg(dst, Value::Int(val as i32));
                    frame.pc = next_pc;
                }
                DalvikInstruction::Const { dst, val } => {
                    frame.set_reg(dst, Value::Int(val));
                    frame.pc = next_pc;
                }
                DalvikInstruction::ConstHigh16 { dst, val } => {
                    frame.set_reg(dst, Value::Int(val));
                    frame.pc = next_pc;
                }
                DalvikInstruction::ConstString { dst, string_idx } => {
                    frame.set_reg(dst, Value::Ref(Some(string_idx as usize)));
                    frame.pc = next_pc;
                }
                DalvikInstruction::Goto { offset } => {
                    let target_pc = (frame.pc as isize) + (offset as isize);
                    if target_pc < 0 || (target_pc as usize) > insns.len() {
                        return ExecutionResult::VerifyError(format!("Goto branch out of bounds: target={}", target_pc));
                    }
                    frame.pc = target_pc as usize;
                }
                DalvikInstruction::IfEq { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() == frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfNe { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() != frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfLt { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() < frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfGe { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() >= frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfGt { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() > frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfLe { v1, v2, offset } => {
                    if frame.get_reg(v1).as_int() <= frame.get_reg(v2).as_int() {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfEqz { v, offset } => {
                    if frame.get_reg(v).as_int() == 0 {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::IfNez { v, offset } => {
                    if frame.get_reg(v).as_int() != 0 {
                        frame.pc = ((frame.pc as isize) + (offset as isize)) as usize;
                    } else {
                        frame.pc = next_pc;
                    }
                }
                DalvikInstruction::AddInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int().wrapping_add(frame.get_reg(src2).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::SubInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int().wrapping_sub(frame.get_reg(src2).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::MulInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int().wrapping_mul(frame.get_reg(src2).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::DivInt { dst, src1, src2 } => {
                    let b = frame.get_reg(src2).as_int();
                    if b == 0 {
                        return ExecutionResult::DivideByZero;
                    }
                    let res = frame.get_reg(src1).as_int().wrapping_div(b);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::RemInt { dst, src1, src2 } => {
                    let b = frame.get_reg(src2).as_int();
                    if b == 0 {
                        return ExecutionResult::DivideByZero;
                    }
                    let res = frame.get_reg(src1).as_int().wrapping_rem(b);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::AndInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int() & frame.get_reg(src2).as_int();
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::OrInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int() | frame.get_reg(src2).as_int();
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::XorInt { dst, src1, src2 } => {
                    let res = frame.get_reg(src1).as_int() ^ frame.get_reg(src2).as_int();
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::AddInt2Addr { dst, src } => {
                    let res = frame.get_reg(dst).as_int().wrapping_add(frame.get_reg(src).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::SubInt2Addr { dst, src } => {
                    let res = frame.get_reg(dst).as_int().wrapping_sub(frame.get_reg(src).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::MulInt2Addr { dst, src } => {
                    let res = frame.get_reg(dst).as_int().wrapping_mul(frame.get_reg(src).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::DivInt2Addr { dst, src } => {
                    let b = frame.get_reg(src).as_int();
                    if b == 0 {
                        return ExecutionResult::DivideByZero;
                    }
                    let res = frame.get_reg(dst).as_int().wrapping_div(b);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::AddIntLit8 { dst, src, lit } => {
                    let res = frame.get_reg(src).as_int().wrapping_add(lit as i32);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::RsubIntLit8 { dst, src, lit } => {
                    let res = (lit as i32).wrapping_sub(frame.get_reg(src).as_int());
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::MulIntLit8 { dst, src, lit } => {
                    let res = frame.get_reg(src).as_int().wrapping_mul(lit as i32);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::DivIntLit8 { dst, src, lit } => {
                    let b = lit as i32;
                    if b == 0 {
                        return ExecutionResult::DivideByZero;
                    }
                    let res = frame.get_reg(src).as_int().wrapping_div(b);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::AddIntLit16 { dst, src, lit } => {
                    let res = frame.get_reg(src).as_int().wrapping_add(lit as i32);
                    frame.set_reg(dst, Value::Int(res));
                    frame.pc = next_pc;
                }
                DalvikInstruction::InvokeStatic { method_idx, args } => {
                    let arg_values: Vec<Value> = args.iter().map(|reg| frame.get_reg(*reg)).collect();
                    return_result_register = Value::Int(method_idx as i32 + arg_values.len() as i32);
                    frame.pc = next_pc;
                }
                DalvikInstruction::InvokeVirtual { method_idx, args }
                | DalvikInstruction::InvokeDirect { method_idx, args } => {
                    let arg_values: Vec<Value> = args.iter().map(|reg| frame.get_reg(*reg)).collect();
                    return_result_register = Value::Int(method_idx as i32 + arg_values.len() as i32);
                    frame.pc = next_pc;
                }
                DalvikInstruction::RawUnknown { opcode, unit_count } => {
                    println!("[DexVm Warning] Skipping unimplemented Dalvik opcode 0x{:02x}", opcode);
                    frame.pc += unit_count;
                }
                _ => {
                    frame.pc = next_pc;
                }
            }
        }

        ExecutionResult::Halted
    }
}
