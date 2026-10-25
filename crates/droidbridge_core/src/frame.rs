use crate::value::Value;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub registers: Vec<Value>,
    pub pc: usize,
    pub ins_size: usize,
    pub registers_size: usize,
}

impl CallFrame {
    pub fn new(registers_size: usize, ins_size: usize, args: &[Value]) -> Self {
        let mut registers = vec![Value::default(); registers_size];

        // Arguments are placed in the last `ins_size` registers (v(N - ins_size) .. vN)
        let param_start = if registers_size >= ins_size {
            registers_size - ins_size
        } else {
            0
        };

        for (i, arg) in args.iter().enumerate().take(ins_size) {
            if param_start + i < registers.len() {
                registers[param_start + i] = arg.clone();
            }
        }

        Self {
            registers,
            pc: 0,
            ins_size,
            registers_size,
        }
    }

    pub fn get_reg(&self, reg: u8) -> Value {
        self.registers.get(reg as usize).cloned().unwrap_or(Value::Null)
    }

    pub fn set_reg(&mut self, reg: u8, val: Value) {
        let idx = reg as usize;
        if idx < self.registers.len() {
            self.registers[idx] = val;
        }
    }
}
