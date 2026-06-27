#![allow(dead_code, unused_variables)]

use crate::{core::{Core, Cpsr, Mode}, decode::{ArmDecoder, ThumbDecoder}, instruction::{Condition, CpsrFields, DataOperation, HalfwordTransferOffset, Instruction, Operand2, Operation, Register, Shift, ShiftCount, ShiftType, WordTransferOffset}};

pub mod tests;
pub mod core;
pub mod instruction;
pub mod decode;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AccessSize {
    Byte,
    Half,
    Word,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CycleType {
    Internal,
    NonSequential,
    Sequential,
}

pub trait Bus {
    fn cycle(&mut self, addr: u32, data: u32, size: AccessSize, ty: CycleType, rw: bool, cpu: &Arm7TDMI) -> u32;
}




pub struct Arm7TDMI {
    core: Core,
    pipeline: [u32; 3],
    pipeline_addrs: [u32; 3],

    last_cycle: (u32, AccessSize, bool, bool),
}
impl Arm7TDMI {
    pub fn new() -> Self {
        Self {
            core: Core::new(),
            pipeline: [0; 3],
            pipeline_addrs: [0; 3],

            last_cycle: (0xDEADBEEF, AccessSize::Word, false, true),
        }
    }
    pub fn init_test(core: Core, pipeline: [u32; 2], base_addr: u32) -> Self {
        let t = core.cpsr.t();
        let instr0 = decode::decode(pipeline[0], t);

        let increment = if t { 2 } else { 4 };
        let addr0 = base_addr;
        let addr1 = base_addr + increment;
        let addr2 = base_addr + increment * 2;

        Self {
            core,
            pipeline: [pipeline[0], pipeline[1], 0],
            pipeline_addrs: [addr0, addr1, addr2],

            last_cycle: (0xDEADBEEF, AccessSize::Word, false, true),
        }
    }

    pub fn core(&self) -> &Core {
        &self.core
    }
    pub fn pipeline(&self) -> [u32; 3] {
        self.pipeline
    }

    pub fn curr_instr(&self) -> (u32, Instruction) {
        let addr = self.pipeline_addrs[0];
        let instr = decode::decode(self.pipeline[0], self.core.cpsr.t());
        (addr, instr)
    }

    pub fn run(&mut self, bus: &mut impl Bus) {
        let instr = decode::decode(self.pipeline[0], self.core.cpsr.t());

        if !self.is_condition_true(instr.condition) {
            self.fetch(bus);
            self.inc_pc();
            self.advance_pipeline();
            return;
        }

        match instr.operation {
            Operation::Bx(register) => self.exec_bx(register, bus),
            Operation::B(offset) => self.exec_b(offset, false, bus),
            Operation::Bl(offset) => self.exec_b(offset, true, bus),
            Operation::Data { operation, s, rd, rn, operand2 } => self.exec_data(operation, s, rd, rn, operand2, bus),
            Operation::MoveFromCpsr(register) => self.exec_mrs(register, false, bus),
            Operation::MoveFromSpsr(register) => self.exec_mrs(register, true, bus),
            Operation::MoveToCpsr(cpsr_fields, register) => self.exec_msr(register, cpsr_fields, false, bus),
            Operation::MoveToSpsr(cpsr_fields, register) => self.exec_msr(register, cpsr_fields, true, bus),
            Operation::MoveToCpsrf(operand2) => todo!(),
            Operation::MoveToSpsrf(operand2) => todo!(),
            Operation::Mul { s, rd, rm, rs } => self.exec_mul(rd, Register(0), rs, rm, false, s, bus),
            Operation::Mla { s, rd, rm, rs, rn } => self.exec_mul(rd, rn, rs, rm, true, s, bus),
            Operation::Mull { s, u, a, rd_lo, rd_hi, rm, rs } => self.exec_mull(rd_lo, rd_hi, rs, rm, a, s, u, bus),
            Operation::Nop => todo!(),
            Operation::WordTransfer { p, u, b, w, l, rn, rd, offset } => todo!(),
            Operation::HalfwordTransfer { p, u, w, l, s, h, rn, rd, offset } => todo!(),
            Operation::BlockTransfer { p, u, s, w, l, rn, register_list } => todo!(),
            Operation::Swap { b, rn, rd, rm } => todo!(),
            Operation::Swi => self.exec_swi(bus),
            Operation::CoprocessorData { cp_operation, crn, crd, cp_number, cp, crm } => todo!(),
            Operation::CoprocessorMemoryTransfer { p, u, n, w, l, rn, crd, cp_number, offset } => todo!(),
            Operation::CoprocessorRegisterTransfer { l, cp_opc, crn, rd, cp_number, cp, crm } => todo!(),
            Operation::Undefined => todo!(),
        }
    }
    fn is_condition_true(&self, cc: Condition) -> bool {
        let cpsr = self.core.cpsr;
        match cc {
            Condition::Equal => cpsr.z(),
            Condition::NotEqual => !cpsr.z(),
            Condition::UnsignedHigherOrSame => cpsr.c(),
            Condition::UnsignedLower => !cpsr.c(),
            Condition::Negative => cpsr.n(),
            Condition::PositiveOrZero => !cpsr.n(),
            Condition::Overflow => cpsr.v(),
            Condition::NoOverflow => !cpsr.v(),
            Condition::UnsignedHigher => cpsr.c() && !cpsr.z(),
            Condition::UnsignedLowerOrSame => !cpsr.c() || cpsr.z(),
            Condition::GreaterOrEqual => cpsr.n() == cpsr.v(),
            Condition::LessThan => cpsr.n() != cpsr.v(),
            Condition::GreaterThan => !cpsr.z() && cpsr.n() == cpsr.v(),
            Condition::LessThanOrEqual => cpsr.z() || cpsr.n() != cpsr.v(),
            Condition::Always => true,
            Condition::Reserved => todo!(),
        }
    }

    fn advance_pipeline(&mut self) {
        self.pipeline[0] = self.pipeline[1];
        self.pipeline[1] = self.pipeline[2];
        self.pipeline_addrs[0] = self.pipeline_addrs[1];
        self.pipeline_addrs[1] = self.pipeline_addrs[2];
    }
    fn fetch(&mut self, bus: &mut impl Bus) {
        let t = self.core.cpsr.t();
        let mask = if t { !1 } else { !3 };
        let addr = self.core[Register::PC] & mask;

        let word = if t {
            self.read(addr, AccessSize::Half, bus)
        }
        else {
            self.read(addr, AccessSize::Word, bus)
        };
        
        self.pipeline_addrs[2] = addr;
        self.pipeline[2] = word;
    }
    fn inc_pc(&mut self) {
        let l = self.core.cpsr.l();
        self.core[Register::PC] = self.core[Register::PC].wrapping_add(l);
    }
    fn refill_pipe(&mut self, bus: &mut impl Bus) {
        self.fetch(bus);
        self.inc_pc();
        self.advance_pipeline();
        self.fetch(bus);
        self.inc_pc();
        self.advance_pipeline();
    }

    fn exec_b(&mut self, offset: i32, link: bool, bus: &mut impl Bus) {
        let tgt = self.core[Register::PC].wrapping_add_signed(offset);
        self.fetch(bus);
        if link {
            let l = self.core.cpsr.l();
            let ret = self.pipeline_addrs[0].wrapping_add(l);
            self.core[Register::LINK] = ret;
        }
        self.core[Register::PC] = tgt;
        self.refill_pipe(bus);
    }
    fn exec_bx(&mut self, rn: Register, bus: &mut impl Bus) {
        let tgt = self.core[rn] & !1;
        let new_t = self.core[rn] & 1 != 0;
        self.fetch(bus);


        self.core.cpsr.set_t(new_t);
        self.core[Register::PC] = tgt;
        self.refill_pipe(bus);
    }
    fn exec_data(&mut self, op: DataOperation, s: bool, rd: Register, rn: Register, op2: Operand2, bus: &mut impl Bus) {
        self.fetch(bus);
        let stalls = op2.has_register_shift();
        let flush = op.has_result() && rd == Register::PC;
        if stalls {
            self.inc_pc();
            self.do_data_operation(op, s, rd, rn, op2);
            self.internal(bus);
        }
        else {
            self.do_data_operation(op, s, rd, rn, op2);
            if !flush {
                self.inc_pc(); 
            }
        }

        if flush {
            self.refill_pipe(bus);
        }
        else {
            self.advance_pipeline();
        }
    }
    fn exec_mrs(&mut self, rd: Register, spsr: bool, bus: &mut impl Bus) {
        self.fetch(bus);
        

        if spsr {
            self.core[rd] = self.core.spsr().0;
        }
        else {
            self.core[rd] = self.core.cpsr.0;
        }


        self.inc_pc();
        self.advance_pipeline();
    }
    fn exec_msr(&mut self, rd: Operand2, fields: CpsrFields, spsr: bool, bus: &mut impl Bus) {
        self.fetch(bus);

        let user = self.core.cpsr.mode() == Mode::User;
        let (data, _) = self.eval_op2(rd);

        self.inc_pc();

        if spsr {
            let mut spsr = self.core.spsr();
            spsr.msr(data, fields, user);
            self.core.set_spsr(spsr);
        }
        else {
            self.core.cpsr.msr(data, fields, user);
        }


        self.advance_pipeline();
    }
    fn exec_mul(&mut self, rd: Register, rn: Register, rs: Register, rm: Register, a: bool, s: bool, bus: &mut impl Bus) {
        self.fetch(bus);
        self.inc_pc();
        
        let add_in = if a { self.core[rn] } else { 0 };
        (self.core[rd], _) = self.core[rm].carrying_mul(self.core[rs], add_in);
        if s {
            self.set_regular_flags(self.core[rd], None);
        }

        self.advance_pipeline();
    }
    fn exec_mull(&mut self, rdlo: Register, rdhi: Register, rs: Register, rm: Register, a: bool, s: bool, u: bool, bus: &mut impl Bus) {
        self.fetch(bus);
        self.inc_pc();

        let (lo, hi);

        let add = if a {
            self.core[rdlo] as u64 | ((self.core[rdhi] as u64) << 32)
        }
        else {
            0
        };

        if !u {
            let rs = self.core[rs] as u64;
            let rm = self.core[rm] as u64;
            let product = rm * rs + add;
            lo = product as u32;
            hi = (product >> 32) as u32;
        }
        else {
            let rs = self.core[rs].cast_signed() as i64;
            let rm = self.core[rm].cast_signed() as i64;
            let product = rm * rs + add.cast_signed();
            let product = product as u64;
            lo = product as u32;
            hi = (product >> 32) as u32;
        }
        


        self.core[rdlo] = lo;
        self.core[rdhi] = hi;
        if s {
            self.core.cpsr.set_z(lo == 0 && hi == 0);
            self.core.cpsr.set_n(hi.cast_signed().is_negative());
        }

        self.advance_pipeline();
    }
    fn exec_swi(&mut self, bus: &mut impl Bus) {
        self.fetch(bus);

        let l = self.core.cpsr.l();

        self.core.spsr_svc = self.core.cpsr;
        self.core.regs_svc[1] = self.core[Register::PC].wrapping_sub(l);
        self.core[Register::PC] = 8;
        self.core.cpsr.set_mode(Mode::Supervisor);
        self.core.cpsr.set_i(true);


        self.refill_pipe(bus);
    }


    fn get_m_addrs(&self, p: bool, u: bool, rn: Register, reg_list: u16) -> (u32, u32) {
        let base = self.core[rn];
        let offset = reg_list.count_ones() * 4;
        let addr = match (p, u) {
            (true, true) => base.wrapping_add(4),
            (true, false) => base.wrapping_sub(offset),
            (false, true) => base,
            (false, false) => base.wrapping_sub(offset).wrapping_add(4),
        };

        let last = addr.wrapping_add(offset - 4);
        let wb = match (p, u) {
            (true, true) => last,
            (true, false) => addr,
            (false, true) => last.wrapping_add(4),
            (false, false) => addr.wrapping_sub(4),
        };


        println!("{base:0>8x}, {offset} => {addr:0>8x}--{wb:0>8x}");
        (addr, wb)
    }
    fn do_block_writeback(&mut self, usr: bool, rn: Register, wb: u32) {
        if usr {
            self.core.regs[rn.0 as usize] = wb;
        }
        else {
            self.core[rn] = wb;
        }
    }

    fn do_data_operation(&mut self, op: DataOperation, s: bool, rd: Register, rn: Register, op2: Operand2) {
        let a: u32 = self.core[rn];
        let (b, bc) = self.eval_op2(op2);

        match op {
            DataOperation::And => {
                let result = a & b;
                self.core[rd] = result;
                if s {
                    self.set_regular_flags(result, bc);
                }
            }
            DataOperation::Eor => {
                let result = a ^ b;
                self.core[rd] = result;
                if s {
                    self.set_regular_flags(result, bc);
                }
            }
            DataOperation::Sub => {
                let (sum, c, v) = sub(a, b, true);
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Rsb => {
                let (a, b) = (b, a);
                let (sum, c, v) = sub(a, b, true);
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Add => {
                let (sum, c, v) = add(a, b, false);
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Adc => {
                let (sum, c, v) = add(a, b, self.core.cpsr.c());
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Sbc => {
                let (sum, c, v) = sub(a, b, self.core.cpsr.c());
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Rsc => {
                let (a, b) = (b, a);
                let (sum, c, v) = sub(a, b, self.core.cpsr.c());
                self.core[rd] = sum;
                if s { 
                    self.set_regular_flags(sum, None);
                    self.core.cpsr.set_v(v);
                    self.core.cpsr.set_c(c);
                }
            }
            DataOperation::Tst => {
                let result = a & b;
                self.set_regular_flags(result, bc);
            }
            DataOperation::Teq => {
                let result = a ^ b;
                self.set_regular_flags(result, bc);
            }
            DataOperation::Cmp => {
                let (diff, c, v) = sub(a, b, true);
                self.set_regular_flags(diff, None);
                self.core.cpsr.set_v(v);
                self.core.cpsr.set_c(c);
            }
            DataOperation::Cmn => {
                let (sum, c, v) = add(a, b, false);
                self.set_regular_flags(sum, None);
                self.core.cpsr.set_v(v);
                self.core.cpsr.set_c(c);
            }
            DataOperation::Orr => {
                let result = a | b;
                self.core[rd] = result;
                if s {
                    self.set_regular_flags(result, bc);
                }
            }
            DataOperation::Mov => {
                self.core[rd] = b;
                if s {
                    self.set_regular_flags(self.core[rd], bc);
                }
            }
            DataOperation::Bic => {
                let result = a & !b;
                self.core[rd] = result;
                if s {
                    self.set_regular_flags(result, bc);
                }
            }
            DataOperation::Mvn => {
                self.core[rd] = !b;
                if s {
                    self.set_regular_flags(self.core[rd], bc);
                }
            }
        }

        if rd == Register::PC && s {
            self.core.cpsr = self.core.spsr();
        }
    }
    fn set_regular_flags(&mut self, data: u32, bc: Option<bool>) {
            self.core.cpsr.set_n(data.cast_signed().is_negative());
            self.core.cpsr.set_z(data == 0);
            if let Some(c) = bc {
                self.core.cpsr.set_c(c);
            }
    }
    fn eval_op2(&mut self, op2: Operand2) -> (u32, Option<bool>) {
        match op2 {
            Operand2::Register(shift) => self.eval_shift(shift),
            Operand2::Immediate(imm, shift) => {
                let (v, bc) = ror(imm as u32, shift * 2);

                if shift == 0 {
                    (v, None)
                }
                else {
                    (v, Some(bc))
                }
            }
        }
    }
    fn eval_shift(&mut self, shift: Shift) -> (u32, Option<bool>) {
        let base = self.core[shift.rm];
        match shift.shift_count {
            ShiftCount::Constant(count) => self.eval_shift_imm(base, shift.shift_type, count),
            ShiftCount::Register(register) => self.eval_shift_rm(base, register, shift.shift_type),
        }
    }
    fn eval_shift_imm(&mut self, base: u32, shift_ty: ShiftType, count: u8) -> (u32, Option<bool>) {
        match shift_ty {
            ShiftType::LogicalLeft => {
                if count == 0 {
                    (base, None)
                }
                else {
                    let (v, c) = lsl(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::LogicalRight => {
                if count == 0 {
                    let c = base.cast_signed().is_negative();
                    (0, Some(c))
                }
                else {
                    let (v, c) = lsr(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::ArithmeticRight => {
                if count == 0 {
                    let c = base.cast_signed().is_negative();
                    let out = if c { 0xFFFFFFFF } else { 0 };
                    (out, Some(c))
                }
                else {
                    let (v, c) = asr(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::RotateRight => {
                if count == 0 {
                    let new_c = base & 1 != 0;
                    let old_c = self.core.cpsr.c();
                    let old_c = if old_c { 0x80000000 } else { 0 };
                    let v = base >> 1 | old_c;
                    (v, Some(new_c))
                }
                else {
                    let (v, bc) = ror(base, count);
                    (v, Some(bc))
                }
            }
        }
    }
    fn eval_shift_rm(&mut self, base: u32, rs: Register, shift_ty: ShiftType) -> (u32, Option<bool>) {
        let count = self.core[rs];
        let l = self.core.cpsr.l();
        let count = if rs  == Register::PC { count - l } else { count };
        let count = count as u8;
        if count == 0 {
            return (base, None);
        }

        match shift_ty {
            ShiftType::LogicalLeft => {
                if count == 32 {
                    (0, Some(base & 1 != 0))
                }
                else if count > 32 {
                    (0, Some(false))
                }
                else {
                    let (v, c) = lsl(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::LogicalRight => {
                if count == 32 {
                    (0, Some(base.cast_signed().is_negative()))
                }
                else if count > 32 {
                    (0, Some(false))
                }
                else {
                    let (v, c) = lsr(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::ArithmeticRight => {
                if count >= 32 {
                    let sign = base.cast_signed().is_negative();
                    let v = if sign { 0xFFFFFFFF } else { 0 };
                    (v, Some(sign))
                }
                else {
                    let (v, c) = asr(base, count);
                    (v, Some(c))
                }
            }
            ShiftType::RotateRight => {
                let mut count = count;
                while count > 32 {
                    count -= 32;
                }
                
                if count == 32 {
                    let c = base.cast_signed().is_negative();
                    (base, Some(c))
                }
                else {
                    let (v, c) = ror(base, count);
                    (v, Some(c))
                }
            }
        }
    }
    fn eval_word_transfer_addr(&mut self, p: bool, u: bool, w: bool, rn: Register, offset: WordTransferOffset) -> (u32, u32, bool) {
        let base = self.core[rn];
        let offset = match offset {
            WordTransferOffset::Immediate(imm) => imm as u32,
            WordTransferOffset::Shift(shift) => self.eval_shift(shift).0
        };

        let addr = if u { base.wrapping_add(offset) } else { base.wrapping_sub(offset) };

        if p && w {
            (addr, addr, true)
        }
        else if p {
            (addr, base, false)
        }
        else {
            (base, addr, true)
        }
    }
    fn eval_halfword_transfer_addr(&mut self, p: bool, u: bool, w: bool, rn: Register, offset: HalfwordTransferOffset) -> (u32, u32, bool) {
        let base = self.core[rn];
        let offset = match offset {
            HalfwordTransferOffset::Immediate(imm) => imm as u32,
            HalfwordTransferOffset::Register(rm) => self.core[rm],
        };

        let addr = if u { base.wrapping_add(offset) } else { base.wrapping_sub(offset) };

        if p && w {
            (addr, addr, true)
        }
        else if p {
            (addr, base, false)
        }
        else {
            (base, addr, true)
        }
    }

    fn cycle(&mut self, addr: u32, data: u32, size: AccessSize, is_access: bool, rw: bool, bus: &mut impl Bus) -> u32 {
        let (last_addr, last_size, last_is_access, last_rw) = self.last_cycle;

        let inc = if size == AccessSize::Half { 2 } else { 4 };
        let is_seq = size != AccessSize::Byte && size == last_size && (addr == last_addr || addr == last_addr.wrapping_add(inc)) && rw == last_rw
            && is_access && last_is_access;

        let ty = match (is_access, is_seq) {
            (false, _) => CycleType::Internal,
            (true, true) => CycleType::Sequential,
            (true, false) => CycleType::NonSequential,
        };

        let rw = if is_access { rw } else { true };
        let data = bus.cycle(addr, data, size, ty, rw, self);

        self.last_cycle = (addr, size, true, rw);

        data
    }
    fn read(&mut self, addr: u32, size: AccessSize, bus: &mut impl Bus) -> u32 {
        self.cycle(addr, 0, size, true, true, bus)
    }
    fn write(&mut self, addr: u32, data: u32, size: AccessSize, bus: &mut impl Bus) {
        self.cycle(addr, data, size, true, false, bus);
    }
    fn internal(&mut self, bus: &mut impl Bus) {
        let size = if self.core.cpsr.t() { AccessSize::Half } else { AccessSize::Word };
        self.cycle(self.core[Register::PC], 0, size, false, true, bus);
    }
}


fn add(a: u32, b: u32, c: bool) -> (u32, bool, bool) {
    let (sum, c) = a.carrying_add(b, c);
    let v = overflow(a, b, sum);
    (sum, c, v)
}
fn sub(a: u32, b: u32, c: bool) -> (u32, bool, bool) {
    add(a, !b, c)
}
fn overflow(a: u32, b: u32, s: u32) -> bool {
    let a = a as i32;
    let b = b as i32;
    let s = s as i32;
    let a = a.is_negative();
    let b = b.is_negative();
    let s = s.is_negative();

    let v = a == b && a != s;
    v
}

fn lsl(base: u32, count: u8) -> (u32, bool) {
    let n = 32 - count as u32;
                    let mask = 1 << n;
                    let c = base & mask != 0;
                    let out = base << count;
                    (out, c)
}
fn lsr(base: u32, count: u8) -> (u32, bool) {
    let n = count as u32 - 1;
                    let mask = 1 << n;
                    let c = base & mask != 0;
                    let out = base >> count;
                    (out, c)
}
fn asr(base: u32, count: u8) -> (u32, bool) {
let n = count as u32 - 1;
                    let mask = 1 << n;
                    let c = base & mask != 0;
                    let out = (base.cast_signed() >> count).cast_unsigned();
                    (out, c)
}
fn ror(base: u32, shift: u8) -> (u32, bool) {
                let mut v = base;
                let mut bc = false;

                // TODO: This is shit
                for _ in 0..shift {
                    bc = v & 1 != 0;
                    v = v.rotate_right(1);
                }

    (v, bc)
}
