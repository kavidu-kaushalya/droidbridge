#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DalvikInstruction {
    Nop,
    Move { dst: u8, src: u8 },
    MoveResult { dst: u8 },
    ReturnVoid,
    Return { src: u8 },
    ReturnWide { src: u8 },
    ReturnObject { src: u8 },
    Const4 { dst: u8, val: i8 },
    Const16 { dst: u8, val: i16 },
    Const { dst: u8, val: i32 },
    ConstHigh16 { dst: u8, val: i32 },
    ConstString { dst: u8, string_idx: u16 },
    Goto { offset: i16 },
    IfEq { v1: u8, v2: u8, offset: i16 },
    IfNe { v1: u8, v2: u8, offset: i16 },
    IfLt { v1: u8, v2: u8, offset: i16 },
    IfGe { v1: u8, v2: u8, offset: i16 },
    IfGt { v1: u8, v2: u8, offset: i16 },
    IfLe { v1: u8, v2: u8, offset: i16 },
    IfEqz { v: u8, offset: i16 },
    IfNez { v: u8, offset: i16 },
    IfLtz { v: u8, offset: i16 },
    IfGez { v: u8, offset: i16 },
    IfGtz { v: u8, offset: i16 },
    IfLez { v: u8, offset: i16 },
    AddInt { dst: u8, src1: u8, src2: u8 },
    SubInt { dst: u8, src1: u8, src2: u8 },
    MulInt { dst: u8, src1: u8, src2: u8 },
    DivInt { dst: u8, src1: u8, src2: u8 },
    RemInt { dst: u8, src1: u8, src2: u8 },
    AndInt { dst: u8, src1: u8, src2: u8 },
    OrInt { dst: u8, src1: u8, src2: u8 },
    XorInt { dst: u8, src1: u8, src2: u8 },
    AddInt2Addr { dst: u8, src: u8 },
    SubInt2Addr { dst: u8, src: u8 },
    MulInt2Addr { dst: u8, src: u8 },
    DivInt2Addr { dst: u8, src: u8 },
    AddIntLit8 { dst: u8, src: u8, lit: i8 },
    RsubIntLit8 { dst: u8, src: u8, lit: i8 },
    MulIntLit8 { dst: u8, src: u8, lit: i8 },
    DivIntLit8 { dst: u8, src: u8, lit: i8 },
    AddIntLit16 { dst: u8, src: u8, lit: i16 },
    InvokeStatic { method_idx: u16, args: Vec<u8> },
    InvokeVirtual { method_idx: u16, args: Vec<u8> },
    InvokeDirect { method_idx: u16, args: Vec<u8> },
    RawUnknown { opcode: u8, unit_count: usize },
}

pub struct InstructionDecoder;

impl InstructionDecoder {
    pub fn decode_next(insns: &[u16], pc: usize) -> Option<(DalvikInstruction, usize)> {
        if pc >= insns.len() {
            return None;
        }

        let u0 = insns[pc];
        let opcode = (u0 & 0xff) as u8;
        let hi8 = (u0 >> 8) as u8;
        let nibble_a = ((u0 >> 8) & 0x0f) as u8;
        let nibble_b = ((u0 >> 12) & 0x0f) as u8;

        match opcode {
            0x00 => Some((DalvikInstruction::Nop, 1)),
            0x01 => Some((DalvikInstruction::Move { dst: nibble_a, src: nibble_b }, 1)),
            0x0a => Some((DalvikInstruction::MoveResult { dst: hi8 }, 1)),
            0x0e => Some((DalvikInstruction::ReturnVoid, 1)),
            0x0f => Some((DalvikInstruction::Return { src: hi8 }, 1)),
            0x10 => Some((DalvikInstruction::ReturnWide { src: hi8 }, 1)),
            0x11 => Some((DalvikInstruction::ReturnObject { src: hi8 }, 1)),
            0x12 => {
                // const/4 vA, #+B
                let raw_val = (nibble_b as i8) << 4 >> 4; // sign extend 4 bits
                Some((DalvikInstruction::Const4 { dst: nibble_a, val: raw_val }, 1))
            }
            0x13 => {
                // const/16 vAA, #+BBBB
                if pc + 1 >= insns.len() { return None; }
                let val = insns[pc + 1] as i16;
                Some((DalvikInstruction::Const16 { dst: hi8, val }, 2))
            }
            0x14 => {
                // const vAA, #+BBBBBBBB
                if pc + 2 >= insns.len() { return None; }
                let lo = insns[pc + 1] as u32;
                let hi = insns[pc + 2] as u32;
                let val = (lo | (hi << 16)) as i32;
                Some((DalvikInstruction::Const { dst: hi8, val }, 3))
            }
            0x15 => {
                // const-high16 vAA, #+BBBB0000
                if pc + 1 >= insns.len() { return None; }
                let val = (insns[pc + 1] as i32) << 16;
                Some((DalvikInstruction::ConstHigh16 { dst: hi8, val }, 2))
            }
            0x1a => {
                // const-string vAA, string@BBBB
                if pc + 1 >= insns.len() { return None; }
                let string_idx = insns[pc + 1];
                Some((DalvikInstruction::ConstString { dst: hi8, string_idx }, 2))
            }
            0x28 => {
                // goto +AA
                let offset = (hi8 as i8) as i16;
                Some((DalvikInstruction::Goto { offset }, 1))
            }
            0x32 => {
                // if-eq vA, vB, +CCCC
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfEq { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x33 => {
                // if-ne vA, vB, +CCCC
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfNe { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x34 => {
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfLt { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x35 => {
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfGe { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x36 => {
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfGt { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x37 => {
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfLe { v1: nibble_a, v2: nibble_b, offset }, 2))
            }
            0x38 => {
                // if-eqz vAA, +BBBB
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfEqz { v: hi8, offset }, 2))
            }
            0x39 => {
                // if-nez vAA, +BBBB
                if pc + 1 >= insns.len() { return None; }
                let offset = insns[pc + 1] as i16;
                Some((DalvikInstruction::IfNez { v: hi8, offset }, 2))
            }
            0x90 => {
                // add-int vAA, vBB, vCC
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::AddInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x91 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::SubInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x92 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::MulInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x93 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::DivInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x94 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::RemInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x95 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::AndInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x96 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::OrInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0x97 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                Some((DalvikInstruction::XorInt { dst: hi8, src1: (u1 & 0xff) as u8, src2: (u1 >> 8) as u8 }, 2))
            }
            0xb0 => Some((DalvikInstruction::AddInt2Addr { dst: nibble_a, src: nibble_b }, 1)),
            0xb1 => Some((DalvikInstruction::SubInt2Addr { dst: nibble_a, src: nibble_b }, 1)),
            0xb2 => Some((DalvikInstruction::MulInt2Addr { dst: nibble_a, src: nibble_b }, 1)),
            0xb3 => Some((DalvikInstruction::DivInt2Addr { dst: nibble_a, src: nibble_b }, 1)),
            0xd0 => {
                // add-int/lit16 vA, vB, #+CCCC
                if pc + 1 >= insns.len() { return None; }
                let lit = insns[pc + 1] as i16;
                Some((DalvikInstruction::AddIntLit16 { dst: nibble_a, src: nibble_b, lit }, 2))
            }
            0xd8 => {
                // add-int/lit8 vAA, vBB, #+CC
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                let src = (u1 & 0xff) as u8;
                let lit = (u1 >> 8) as i8;
                Some((DalvikInstruction::AddIntLit8 { dst: hi8, src, lit }, 2))
            }
            0xd9 => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                let src = (u1 & 0xff) as u8;
                let lit = (u1 >> 8) as i8;
                Some((DalvikInstruction::RsubIntLit8 { dst: hi8, src, lit }, 2))
            }
            0xda => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                let src = (u1 & 0xff) as u8;
                let lit = (u1 >> 8) as i8;
                Some((DalvikInstruction::MulIntLit8 { dst: hi8, src, lit }, 2))
            }
            0xdb => {
                if pc + 1 >= insns.len() { return None; }
                let u1 = insns[pc + 1];
                let src = (u1 & 0xff) as u8;
                let lit = (u1 >> 8) as i8;
                Some((DalvikInstruction::DivIntLit8 { dst: hi8, src, lit }, 2))
            }
            0x71 => {
                // invoke-static {vC, vD, vE, vF, vG}, meth@BBBB
                if pc + 2 >= insns.len() { return None; }
                let method_idx = insns[pc + 1];
                let count = (nibble_b) as usize;
                let u2 = insns[pc + 2];
                let mut args = Vec::new();
                let reg_pool = [
                    (u2 & 0x0f) as u8,
                    ((u2 >> 4) & 0x0f) as u8,
                    ((u2 >> 8) & 0x0f) as u8,
                    ((u2 >> 12) & 0x0f) as u8,
                    nibble_a,
                ];
                for i in 0..count.min(5) {
                    args.push(reg_pool[i]);
                }
                Some((DalvikInstruction::InvokeStatic { method_idx, args }, 3))
            }
            0x6e => {
                // invoke-virtual
                if pc + 2 >= insns.len() { return None; }
                let method_idx = insns[pc + 1];
                let count = (nibble_b) as usize;
                let u2 = insns[pc + 2];
                let mut args = Vec::new();
                let reg_pool = [
                    (u2 & 0x0f) as u8,
                    ((u2 >> 4) & 0x0f) as u8,
                    ((u2 >> 8) & 0x0f) as u8,
                    ((u2 >> 12) & 0x0f) as u8,
                    nibble_a,
                ];
                for i in 0..count.min(5) {
                    args.push(reg_pool[i]);
                }
                Some((DalvikInstruction::InvokeVirtual { method_idx, args }, 3))
            }
            0x70 => {
                // invoke-direct
                if pc + 2 >= insns.len() { return None; }
                let method_idx = insns[pc + 1];
                let count = (nibble_b) as usize;
                let u2 = insns[pc + 2];
                let mut args = Vec::new();
                let reg_pool = [
                    (u2 & 0x0f) as u8,
                    ((u2 >> 4) & 0x0f) as u8,
                    ((u2 >> 8) & 0x0f) as u8,
                    ((u2 >> 12) & 0x0f) as u8,
                    nibble_a,
                ];
                for i in 0..count.min(5) {
                    args.push(reg_pool[i]);
                }
                Some((DalvikInstruction::InvokeDirect { method_idx, args }, 3))
            }
            _ => Some((DalvikInstruction::RawUnknown { opcode, unit_count: 1 }, 1)),
        }
    }
}
