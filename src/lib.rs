use crate::{
    core::{Core, Mode},
    decode::decode,
    instruction::{
        Cc, DataOp, HalfOffset, HiRegOp, Instr, Op, Op2, PsrFields, Reg, Shift, ShiftCount,
        ShiftType, WordOffset,
    },
};

pub mod core;
pub mod decode;
pub mod instruction;
pub mod tests;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Size {
    Byte,
    Half,
    Word,
}

pub trait Bus {
    fn access(
        &mut self,
        addr: u32,
        data: u32,
        rw: bool,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) -> u32;
    fn io(&mut self, addr: u32);

    fn dbg_start_instr(&mut self, instr: Instr, addr: u32, core: &Core);
    fn dbg_load(
        &mut self,
        core: &Core,
        addr: u32,
        data: u32,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    );
    fn dbg_store(&mut self, core: &Core, addr: u32, data: u32, size: Size, seq: bool, lock: bool);
    fn dbg_io(&mut self, core: &Core, addr: u32);

    fn dbg_c_unpredictable(&mut self);
    fn dbg_v_unpredictable(&mut self);
}

pub struct Arm7 {
    core: Core,
    pipe: [(u32, u32); 3],

    next_seq: bool,
}
impl Arm7 {
    pub fn init() -> Self {
        Self {
            core: Core::new(),
            pipe: [(0, 0xDEADBEEF); 3],
            next_seq: false,
        }
    }

    pub fn new(core: Core, pipeline: [u32; 2]) -> Self {
        Self {
            core,
            pipe: [
                (pipeline[0], 0xDEADBEEF),
                (pipeline[1], 0xDEADBEEF),
                (0xDEADBEEF, 0xDEADBEEF),
            ],
            next_seq: false,
        }
    }

    pub fn exec(&mut self, bus: &mut impl Bus) {
        let instr = decode(self.pipe[0].0, self.core.cpsr.t());
        bus.dbg_start_instr(instr, self.pipe[0].1, &self.core);
        if !self.is_cc_true(instr.cc) {
            self.fetch(bus, true);
            self.inc_pc();
            self.advance_pipeline();
        } else {
            #[allow(unused_variables)]
            match instr.op {
                Op::Bx(rn) => self.exec_bx(bus, rn),
                Op::B(offset) => self.exec_b(bus, offset, false),
                Op::Bl(offset) => self.exec_b(bus, offset, true),
                Op::Data { op, s, rd, rn, op2 } => self.exec_data(bus, op, s, rd, rn, op2),
                Op::Mrs { p, rd } => self.exec_mrs(bus, p, rd),
                Op::Msr { p, fields, op2 } => self.exec_msr(bus, p, fields, op2),
                Op::Mul { s, rd, rm, rs } => self.exec_mul(bus, s, false, rd, rm, rs, Reg(0)),
                Op::Mla { s, rd, rm, rs, rn } => self.exec_mul(bus, s, true, rd, rm, rs, rn),
                Op::Mull {
                    s,
                    u,
                    a,
                    rd_lo,
                    rd_hi,
                    rm,
                    rs,
                } => self.exec_mull(bus, s, u, a, rd_lo, rd_hi, rm, rs),
                Op::Nop => todo!(),
                Op::WordTransfer {
                    p,
                    u,
                    b,
                    w,
                    l,
                    rn,
                    rd,
                    offset,
                } => self.exec_lda_sta(bus, p, u, b, w, l, rn, rd, offset),
                Op::HalfwordTransfer {
                    p,
                    u,
                    w,
                    l,
                    s,
                    h,
                    rn,
                    rd,
                    offset,
                } => self.exec_half_transfer(bus, p, u, w, l, s, h, rn, rd, offset),
                Op::BlockTransfer {
                    p,
                    u,
                    s,
                    w,
                    l,
                    rn,
                    regs,
                } => self.exec_ldm_stm(bus, p, u, s, w, l, rn, regs),
                Op::Swap { b, rn, rd, rm } => self.exec_swp(bus, b, rd, rn, rm),
                Op::Swi => self.exec_swi(bus),
                Op::Undefined => todo!(),

                Op::HiRegOps { op, rs, rd } => self.exec_hi_reg_ops(bus, op, rs, rd),
                Op::LoadAddr { sp, rd, offset } => self.exec_load_addr(bus, sp, rd, offset),
                Op::BranchLong { h, offset } => self.exec_thumb_branch(bus, h, offset),
                Op::LdrPc { rd, offset } => self.exec_ldr_pc(bus, rd, offset),
            }
        }
    }

    fn is_cc_true(&self, cc: Cc) -> bool {
        let z = self.core.cpsr.z();
        let c = self.core.cpsr.c();
        let n = self.core.cpsr.n();
        let v = self.core.cpsr.v();

        match cc {
            Cc::Equal => z,
            Cc::NotEqual => !z,
            Cc::UnsignedHigherOrSame => c,
            Cc::UnsignedLower => !c,
            Cc::Negative => n,
            Cc::PositiveOrZero => !n,
            Cc::Overflow => v,
            Cc::NoOverflow => !v,
            Cc::UnsignedHigher => c && !z,
            Cc::UnsignedLowerOrSame => !c || z,
            Cc::GreaterOrEqual => n == v,
            Cc::LessThan => n != v,
            Cc::GreaterThan => !z && (n == v),
            Cc::LessThanOrEqual => z || (n != v),
            Cc::Always => true,
            Cc::Reserved => unreachable!(),
        }
    }
    fn exec_b(&mut self, bus: &mut impl Bus, offset: i32, link: bool) {
        self.fetch(bus, false);
        let tgt = self.core.regs[15].wrapping_add_signed(offset);
        if link {
            let size = if self.core.cpsr.t() { 2 } else { 4 };
            let ret = self.core.regs[15].wrapping_sub(size);
            self.core.set_reg(Reg::LINK, ret);
        }

        self.core.regs[15] = tgt;
        self.refill_pipe(bus);
    }
    fn exec_bx(&mut self, bus: &mut impl Bus, rn: Reg) {
        self.fetch(bus, false);
        let tgt = self.core.get_reg(rn);
        let next_t = tgt & 1 != 0;
        let tgt = tgt & !1;
        self.core.regs[15] = tgt;
        self.core.cpsr.set_t(next_t);
        self.refill_pipe(bus);
    }
    fn exec_data(&mut self, bus: &mut impl Bus, op: DataOp, s: bool, rd: Reg, rn: Reg, op2: Op2) {
        let stall = op2.has_register_shift();
        let jump = op.has_result() && rd.is_pc();

        let a = self.core.get_reg(rn);
        let a = if rn.is_pc() && stall {
            a.wrapping_add(4)
        } else {
            a
        };
        let (b, c) = self.eval_op2(op2);

        self.fetch(bus, !stall && !jump);
        self.inc_pc();

        if stall {
            self.io(bus, self.core.regs[15], !jump);
        }

        self.do_data_op(op, s, rd, a, b, c);

        if jump {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_lda_sta(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        l: bool,
        rn: Reg,
        rd: Reg,
        offset: WordOffset,
    ) {
        if l {
            self.exec_ldr(bus, p, u, b, w, rn, rd, offset);
        } else {
            self.exec_str(bus, p, u, b, w, rn, rd, offset);
        }
    }
    fn exec_ldr(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        rn: Reg,
        rd: Reg,
        offset: WordOffset,
    ) {
        self.fetch(bus, false);

        let (addr, wb) = self.calc_ldr_address(p, u, rn, offset);
        let size = if b { Size::Byte } else { Size::Word };

        let do_wb = w || !p;
        let flush = rd.is_pc() || rn.is_pc() && do_wb;

        if do_wb {
            self.core.set_reg(rn, wb);
        }
        self.inc_pc();

        let word = self.access(bus, addr, 0, true, size, false, false, false);
        let lo = addr & 3;
        let rot = lo * 8;
        let word = word.rotate_right(rot);

        self.io(bus, self.core.regs[15], !flush);

        if b {
            self.core.set_reg(rd, word & 0xFF);
        } else {
            self.core.set_reg(rd, word);
        }

        if flush {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_str(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        rn: Reg,
        rd: Reg,
        offset: WordOffset,
    ) {
        self.fetch(bus, false);

        let (addr, wb) = self.calc_ldr_address(p, u, rn, offset);
        let size = if b { Size::Byte } else { Size::Word };

        let do_wb = w || !p;
        let flush = rn.is_pc() && do_wb;

        // this is stupid!
        // figure out some pattern?
        // or maybe just never touch it again
        let word = if !p && rd.is_pc() {
            let word = self.core.get_reg(rd).wrapping_add(4);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if !p {
            let word = self.core.get_reg(rd);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w && rd.is_pc() {
            let word = self.core.get_reg(rd).wrapping_add(4);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w && rd == rn {
            let word = self.core.get_reg(rd);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w {
            self.core.set_reg(rn, wb);
            self.inc_pc();
            let word = self.core.get_reg(rd);
            word
        } else {
            self.inc_pc();
            let word = self.core.get_reg(rd);
            word
        };

        let byte = word as u8;
        let word = if b {
            u32::from_le_bytes([byte, byte, byte, byte])
        } else {
            word
        };
        self.access(bus, addr, word, false, size, false, false, false);

        if flush {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_half_transfer(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        w: bool,
        l: bool,
        s: bool,
        h: bool,
        rn: Reg,
        rd: Reg,
        offset: HalfOffset,
    ) {
        if l {
            self.exec_ldrh(bus, p, u, w, s, h, rn, rd, offset);
        } else {
            self.exec_strh(bus, p, u, w, rn, rd, offset);
        }
    }
    fn exec_ldrh(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        w: bool,
        s: bool,
        h: bool,
        rn: Reg,
        rd: Reg,
        offset: HalfOffset,
    ) {
        self.fetch(bus, false);

        let (addr, wb) = self.calc_ldrh_address(p, u, rn, offset);
        let size = if h { Size::Half } else { Size::Byte };

        let do_wb = w || !p;
        let flush = rd.is_pc() || rn.is_pc() && do_wb;

        if do_wb {
            self.core.set_reg(rn, wb);
        }
        self.inc_pc();

        let word = self.access(bus, addr, 0, true, size, false, false, false);
        let lo = addr & 3;

        self.io(bus, self.core.regs[15], !flush);

        match (s, h) {
            (true, true) => {
                let word = match lo {
                    0 => word as i16 as i32 as u32,
                    1 => word.rotate_right(16) as i8 as i32 as u32,
                    2 => word.rotate_right(16) as i16 as i32 as u32,
                    3 => word as i8 as i32 as u32,
                    4.. => unreachable!(),
                };
                self.core.set_reg(rd, word);
            }
            (true, false) => {
                let ror = lo * 8;
                let word = word.rotate_right(ror);
                let word = word as i8 as i32 as u32;
                self.core.set_reg(rd, word);
            }
            (false, true) => {
                let word = match lo {
                    0 => word as u16 as u32,
                    1 => word.rotate_right(16),
                    2 => word.rotate_right(16) as u16 as u32,
                    3 => word,
                    4.. => unreachable!(),
                };
                self.core.set_reg(rd, word);
            }
            (false, false) => unreachable!(),
        }

        if flush {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_strh(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        w: bool,
        rn: Reg,
        rd: Reg,
        offset: HalfOffset,
    ) {
        self.fetch(bus, false);

        let (addr, wb) = self.calc_ldrh_address(p, u, rn, offset);
        let size = Size::Half;

        let do_wb = w || !p;
        let flush = rn.is_pc() && do_wb;

        // this is stupid!
        // figure out some pattern?
        // or maybe just never touch it again
        let word = if !p && rd.is_pc() {
            let word = self.core.get_reg(rd).wrapping_add(4);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if !p {
            let word = self.core.get_reg(rd);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w && rd.is_pc() {
            let word = self.core.get_reg(rd).wrapping_add(4);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w && rd == rn {
            let word = self.core.get_reg(rd);
            self.core.set_reg(rn, wb);
            self.inc_pc();
            word
        } else if w {
            self.core.set_reg(rn, wb);
            self.inc_pc();
            let word = self.core.get_reg(rd);
            word
        } else {
            self.inc_pc();
            let word = self.core.get_reg(rd);
            word
        };

        let half = word as u16;
        let word = (half as u32) | (half as u32) << 16;
        self.access(bus, addr, word, false, size, false, false, false);

        if flush {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_mrs(&mut self, bus: &mut impl Bus, p: bool, rd: Reg) {
        self.fetch(bus, true);

        let src = if p && self.core.cpsr.mode().has_spsr() {
            self.core.curr_spsr().0
        } else {
            self.core.cpsr.0
        };
        self.core.set_reg(rd, src);

        self.inc_pc();
        self.advance_pipeline();
    }
    fn exec_msr(&mut self, bus: &mut impl Bus, p: bool, fields: PsrFields, op2: Op2) {
        self.fetch(bus, true);
        let (src, _) = self.eval_op2(op2);
        self.inc_pc();

        let usr = self.core.cpsr.mode() == Mode::Usr;

        if p && self.core.cpsr.mode().has_spsr() {
            self.core.curr_spsr_mut().do_msr(src, fields, usr, false);
        } else if !p {
            self.core.cpsr.do_msr(src, fields, usr, true);
        };

        self.advance_pipeline();
    }
    fn exec_ldm_stm(
        &mut self,
        bus: &mut impl Bus,
        p: bool,
        u: bool,
        s: bool,
        w: bool,
        l: bool,
        rn: Reg,
        regs: u16,
    ) {
        self.fetch(bus, false);
        let old_pc = self.core.regs[15];

        let (base, wb) = self.calc_ldm_address(p, u, rn, regs);
        self.inc_pc();
        let fake_regs = regs == 0;
        let mut regs = if regs == 0 { 0x8000 } else { regs };

        let has_pc = regs & 0x8000 != 0;
        let usr = (!l || !has_pc) && s;

        let flush = (has_pc && l) || (rn.is_pc() && w);

        let mut step = 0;
        let mut reg_i = 0;

        let mut addr = base;

        while regs != 0 {
            while regs & 1 == 0 {
                regs >>= 1;
                reg_i += 1;
            }
            let reg = Reg(reg_i);

            if !l {
                let word = if usr {
                    self.core.get_reg_usr(reg)
                } else {
                    self.core.get_reg(reg)
                };

                self.access(bus, addr, word, false, Size::Word, false, regs != 1, false);
            }

            if step == 0 && w {
                if usr {
                    self.core.set_reg_usr(rn, wb);
                } else {
                    self.core.set_reg(rn, wb);
                }
            }

            if l {
                let word = self.access(bus, addr, 0, true, Size::Word, false, regs != 1, false);
                let word = if reg.is_pc() && self.core.cpsr.t() && !fake_regs {
                    word & !1
                } else {
                    word
                };

                if usr {
                    self.core.set_reg_usr(reg, word);
                } else {
                    self.core.set_reg(reg, word);
                }
            }

            regs &= !1;
            addr += 4;
            step += 1;
        }

        if l {
            self.io(bus, old_pc, !flush);
        }

        if flush {
            if s && l && has_pc && self.core.cpsr.mode().has_spsr() {
                self.core.cpsr = *self.core.curr_spsr();
            }
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_mul(
        &mut self,
        bus: &mut impl Bus,
        s: bool,
        a: bool,
        rd: Reg,
        rm: Reg,
        rs: Reg,
        rn: Reg,
    ) {
        self.fetch(bus, false);
        self.inc_pc();

        let acc = a;

        let a = self.core.get_reg(rm);
        let b = self.core.get_reg(rs);
        let c = self.core.get_reg(rn);

        let product = a.wrapping_mul(b);
        let addend = if acc { c } else { 0 };
        let result = product.wrapping_add(addend);

        self.core.set_reg(rd, result);
        if s {
            bus.dbg_c_unpredictable();
            self.core.cpsr.set_z(result == 0);
            self.core.cpsr.set_n(result.cast_signed().is_negative());
        }

        if rd.is_pc() {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_mull(
        &mut self,
        bus: &mut impl Bus,
        s: bool,
        u: bool,
        a: bool,
        rd_lo: Reg,
        rd_hi: Reg,
        rm: Reg,
        rs: Reg,
    ) {
        self.fetch(bus, false);
        self.inc_pc();

        let acc = a;

        let a = self.core.get_reg(rm);
        let b = self.core.get_reg(rs);
        let c = self.core.get_reg(rd_lo);
        let d = self.core.get_reg(rd_hi);

        let addend = if acc {
            ((d as u64) << 32) | (c as u64)
        } else {
            0
        };

        let result = if u {
            (a as i32 as i64)
                .wrapping_mul(b as i32 as i64)
                .wrapping_add(addend as i64) as u64
        } else {
            (a as u64).wrapping_mul(b as u64).wrapping_add(addend)
        };

        if s {
            bus.dbg_c_unpredictable();
            bus.dbg_v_unpredictable();
            self.core.cpsr.set_z(result == 0);
            self.core.cpsr.set_n(result.cast_signed().is_negative());
        }

        self.core.set_reg(rd_lo, result as u32);
        self.core.set_reg(rd_hi, (result >> 32) as u32);

        if rd_lo.is_pc() || rd_hi.is_pc() {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_swi(&mut self, bus: &mut impl Bus) {
        self.fetch(bus, false);

        let size = if self.core.cpsr.t() { 2 } else { 4 };
        let ret = self.core.get_reg(Reg::PC).wrapping_sub(size);

        self.core.spsr_svc = self.core.cpsr;
        self.core.cpsr.set_mode(Mode::Svc);
        self.core.cpsr.set_t(false);
        self.core.cpsr.set_i(true);
        self.core.set_reg(Reg::LINK, ret);
        self.core.set_reg(Reg::PC, 8);

        self.refill_pipe(bus);
    }
    fn exec_swp(&mut self, bus: &mut impl Bus, b: bool, rd: Reg, rn: Reg, rm: Reg) {
        self.fetch(bus, false);
        self.inc_pc();

        let addr = self.core.get_reg(rn);
        let src = self.core.get_reg(rm);

        let size = if b { Size::Byte } else { Size::Word };
        let word = self.access(bus, addr, 0, true, size, false, false, false);

        let lo = src as u8;
        let src = if b { u32::from_le_bytes([lo; 4]) } else { src };

        self.access(bus, addr, src, false, size, false, false, true);

        let ror = (addr & 3) * 8;
        let word = word.rotate_right(ror);
        let word = if b { word & 0xFF } else { word };
        self.core.set_reg(rd, word);

        self.io(bus, self.core.get_reg(Reg::PC), true);

        if rd.is_pc() {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn calc_ldr_address(&self, p: bool, u: bool, rn: Reg, offset: WordOffset) -> (u32, u32) {
        let base = self.core.get_reg(rn);
        let offset = match offset {
            WordOffset::Immediate(offset) => offset as u32,
            WordOffset::Shift(shift) => self.eval_shift(shift).0,
        };

        let off = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        if p { (off, off) } else { (base, off) }
    }
    fn calc_ldm_address(&self, p: bool, u: bool, rn: Reg, regs: u16) -> (u32, u32) {
        let base = self.core.get_reg(rn);
        let size = regs.count_ones() * 4;
        let size = if regs == 0 { 0x40 } else { size };

        let off = if u {
            base.wrapping_add(size)
        } else {
            base.wrapping_sub(size)
        };

        if p && u {
            (base.wrapping_add(4), off)
        } else if p {
            (off, off)
        } else if u {
            (base, off)
        } else {
            (off.wrapping_add(4), off)
        }
    }
    fn calc_ldrh_address(&self, p: bool, u: bool, rn: Reg, offset: HalfOffset) -> (u32, u32) {
        let base = self.core.get_reg(rn);
        let offset = match offset {
            HalfOffset::Immediate(offset) => offset as u32,
            HalfOffset::Register(rm) => self.core.get_reg(rm),
        };

        let off = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        if p { (off, off) } else { (base, off) }
    }
    fn do_data_op(&mut self, op: DataOp, s: bool, rd: Reg, a: u32, b: u32, c: bool) {
        let result = match op {
            DataOp::And | DataOp::Tst => {
                let result = a & b;
                self.set_logic_flags(s, result, c);

                result
            }
            DataOp::Eor | DataOp::Teq => {
                let result = a ^ b;
                self.set_logic_flags(s, result, c);

                result
            }
            DataOp::Sub | DataOp::Cmp => {
                let (result, c, v) = sub(a, b, true);
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Rsb => {
                let (result, c, v) = sub(b, a, true);
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Add | DataOp::Cmn => {
                let (result, c, v) = add(a, b, false);
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Adc => {
                let (result, c, v) = add(a, b, self.core.cpsr.c());
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Sbc => {
                let (result, c, v) = sub(a, b, self.core.cpsr.c());
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Rsc => {
                let (result, c, v) = sub(b, a, self.core.cpsr.c());
                self.set_arith_flags(s, result, c, v);
                result
            }
            DataOp::Orr => {
                let result = a | b;
                self.set_logic_flags(s, result, c);

                result
            }
            DataOp::Mov => {
                let result = b;
                self.set_logic_flags(s, result, c);

                result
            }
            DataOp::Bic => {
                let result = a & !b;
                self.set_logic_flags(s, result, c);

                result
            }
            DataOp::Mvn => {
                let result = !b;
                self.set_logic_flags(s, result, c);

                result
            }
        };

        if op.has_result() {
            self.core.set_reg(rd, result);
        }

        if rd.is_pc() && s && self.core.cpsr.mode().has_spsr() && !self.core.cpsr.t() {
            self.core.cpsr = *self.core.curr_spsr();
        }
    }
    fn set_arith_flags(&mut self, s: bool, result: u32, c: bool, v: bool) {
        if !s {
            return;
        }
        self.core.cpsr.set_z(result == 0);
        self.core.cpsr.set_c(c);
        self.core.cpsr.set_v(v);
        self.core.cpsr.set_n(result.cast_signed().is_negative());
    }
    fn set_logic_flags(&mut self, s: bool, result: u32, c: bool) {
        if !s {
            return;
        }
        self.core.cpsr.set_z(result == 0);
        self.core.cpsr.set_c(c);
        self.core.cpsr.set_n(result.cast_signed().is_negative());
    }
    fn eval_op2(&self, op2: Op2) -> (u32, bool) {
        match op2 {
            Op2::Register(shift) => self.eval_shift(shift),
            Op2::Immediate(base, shift) => {
                let out = (base as u32).rotate_right(shift as u32);
                let c = out.cast_signed().is_negative();
                let c = if shift == 0 { self.core.cpsr.c() } else { c };
                (out, c)
            }
        }
    }
    fn eval_shift(&self, shift: Shift) -> (u32, bool) {
        let base = self.core.get_reg(shift.rm);
        let c = self.core.cpsr.c();

        match (shift.shift_count, shift.shift_type) {
            (ShiftCount::Constant(count), ShiftType::LogicalLeft) => lsl_by_imm(base, count, c),
            (ShiftCount::Constant(count), ShiftType::LogicalRight) => lsr_by_imm(base, count, c),
            (ShiftCount::Constant(count), ShiftType::ArithmeticRight) => asr_by_imm(base, count, c),
            (ShiftCount::Constant(count), ShiftType::RotateRight) => ror_by_imm(base, count, c),
            (ShiftCount::Register(rs), _) => {
                let base = if shift.rm.is_pc() {
                    base.wrapping_add(4)
                } else {
                    base
                };
                let count = self.core.get_reg(rs) as u8;
                match shift.shift_type {
                    ShiftType::LogicalLeft => lsl_by_reg(base, count, c),
                    ShiftType::LogicalRight => lsr_by_reg(base, count, c),
                    ShiftType::ArithmeticRight => asr_by_reg(base, count, c),
                    ShiftType::RotateRight => ror_by_reg(base, count, c),
                }
            }
        }
    }

    fn exec_hi_reg_ops(&mut self, bus: &mut impl Bus, op: HiRegOp, rs: Reg, rd: Reg) {
        self.fetch(bus, true);

        let a = self.core.get_reg(rd);
        let b = self.core.get_reg(rs);
        self.inc_pc();

        let (result, c, v) = match op {
            HiRegOp::Add => add(a, b, false),
            HiRegOp::Cmp => sub(a, b, true),
            HiRegOp::Mov => (b, false, false),
        };

        let s = op == HiRegOp::Cmp;

        self.set_arith_flags(s, result, c, v);
        if !s {
            let result = if rd.is_pc() { result & !1 } else { result };
            self.core.set_reg(rd, result);
        }

        if rd.is_pc() && !s {
            self.refill_pipe(bus);
        } else {
            self.advance_pipeline();
        }
    }
    fn exec_load_addr(&mut self, bus: &mut impl Bus, sp: bool, rd: Reg, offset: u8) {
        self.fetch(bus, true);

        let offset = offset as u32 * 4;
        let src = if sp { Reg::STACK } else { Reg::PC };
        let src = self.core.get_reg(src);
        let src = if !sp { src & !3 } else { src };
        let sum = src.wrapping_add(offset);
        self.core.set_reg(rd, sum);

        self.inc_pc();
        self.advance_pipeline();
    }
    fn exec_thumb_branch(&mut self, bus: &mut impl Bus, h: bool, offset: i32) {
        if h {
            self.fetch(bus, false);
            let tgt = self
                .core
                .get_reg(Reg::LINK)
                .wrapping_add_signed((offset & 0x7FF) << 1);
            let tgt = tgt & !1;
            let ret = self.core.get_reg(Reg::PC).wrapping_sub(2) | 1;
            self.core.set_reg(Reg::LINK, ret);
            self.core.set_reg(Reg::PC, tgt);

            self.refill_pipe(bus);
        } else {
            self.fetch(bus, true);
            let pc = self.core.get_reg(Reg::PC);
            let sum = pc.wrapping_add_signed(offset << 12);
            self.core.set_reg(Reg::LINK, sum);
            self.inc_pc();
            self.advance_pipeline();
        }
    }
    fn exec_ldr_pc(&mut self, bus: &mut impl Bus, rd: Reg, offset: u8) {
        self.fetch(bus, false);

        let base = self.core.get_reg(Reg::PC) & !2;
        let offset = offset as u32 * 4;
        let addr = base.wrapping_add(offset);

        let word = self.access(bus, addr, 0, true, Size::Word, false, false, false);
        self.core.set_reg(rd, word);

        self.inc_pc();
        self.io(bus, self.core.get_reg(Reg::PC), true);
        self.advance_pipeline();
    }

    fn fetch(&mut self, bus: &mut impl Bus, seq: bool) {
        let pc = self.core.regs[15];
        let t = self.core.cpsr.t();
        let size = if t { Size::Half } else { Size::Word };

        let mask = if t { !1 } else { !3 };
        let pc = pc & mask;
        let shift = if t && pc % 4 != 0 { 16 } else { 0 };

        let word = self.access(bus, pc, 0, true, size, true, seq, false);
        self.pipe[2].0 = word >> shift;
        self.pipe[2].1 = pc;
    }
    fn inc_pc(&mut self) {
        let by = if self.core.cpsr.t() { 2 } else { 4 };
        let new = self.core.regs[15].wrapping_add(by);
        self.core.regs[15] = new;
    }
    fn advance_pipeline(&mut self) {
        self.pipe[0] = self.pipe[1];
        self.pipe[1] = self.pipe[2];
    }
    fn refill_pipe(&mut self, bus: &mut impl Bus) {
        self.fetch(bus, true);
        self.inc_pc();
        self.advance_pipeline();

        self.fetch(bus, true);
        self.inc_pc();
        self.advance_pipeline();
    }

    fn access(
        &mut self,
        bus: &mut impl Bus,
        addr: u32,
        data: u32,
        rw: bool,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) -> u32 {
        if !rw {
            bus.dbg_store(&self.core, addr, data, size, self.next_seq, lock);
        }
        let out = bus.access(addr, data, rw, size, opc, self.next_seq, lock);
        if rw {
            bus.dbg_load(&self.core, addr, out, size, opc, self.next_seq, lock);
        }

        self.next_seq = seq;
        out
    }
    fn io(&mut self, bus: &mut impl Bus, addr: u32, seq: bool) {
        bus.dbg_io(&self.core, addr);
        bus.io(addr);
        self.next_seq = seq;
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
fn ror(base: u32, count: u8) -> (u32, bool) {
    let mut v = base;
    let mut bc = false;

    // TODO: This is shit
    for _ in 0..count {
        bc = v & 1 != 0;
        v = v.rotate_right(1);
    }

    (v, bc)
}

fn lsl_by_imm(base: u32, count: u8, c: bool) -> (u32, bool) {
    if count == 0 {
        (base, c)
    } else {
        lsl(base, count)
    }
}
fn lsr_by_imm(base: u32, count: u8, c: bool) -> (u32, bool) {
    let _ = c;
    if count == 0 {
        let c = base.cast_signed().is_negative();
        let out = 0;
        (out, c)
    } else {
        lsr(base, count)
    }
}
fn asr_by_imm(base: u32, count: u8, c: bool) -> (u32, bool) {
    let _ = c;
    if count == 0 {
        let c = base.cast_signed().is_negative();
        let out = if c { (-1i32).cast_unsigned() } else { 0 };
        (out, c)
    } else {
        asr(base, count)
    }
}
fn ror_by_imm(base: u32, count: u8, c: bool) -> (u32, bool) {
    if count == 0 {
        let co = base & 1 != 0;
        let out = base.rotate_right(1);
        let out = out & !(1 << 31);
        let out = out | (c as u32) << 31;
        (out, co)
    } else {
        ror(base, count)
    }
}

fn lsl_by_reg(base: u32, count: u8, c: bool) -> (u32, bool) {
    if count == 0 {
        (base, c)
    } else if count < 32 {
        lsl(base, count)
    } else if count == 32 {
        let c = base & 1 != 0;
        (0, c)
    } else {
        (0, false)
    }
}
fn lsr_by_reg(base: u32, count: u8, c: bool) -> (u32, bool) {
    if count == 0 {
        (base, c)
    } else if count < 32 {
        lsr(base, count)
    } else if count == 32 {
        let c = base.cast_signed().is_negative();
        (0, c)
    } else {
        (0, false)
    }
}
fn asr_by_reg(base: u32, count: u8, c: bool) -> (u32, bool) {
    if count == 0 {
        (base, c)
    } else if count < 32 {
        asr(base, count)
    } else {
        let c = base.cast_signed().is_negative();
        let out = if c { u32::MAX } else { 0 };
        (out, c)
    }
}
fn ror_by_reg(base: u32, count: u8, c: bool) -> (u32, bool) {
    let mut count = count;
    while count > 32 {
        count -= 32;
    }

    if count == 0 {
        (base, c)
    } else if count < 32 {
        ror(base, count)
    } else if count == 32 {
        let c = base.cast_signed().is_negative();
        (base, c)
    } else {
        unreachable!()
    }
}
