use std::io::{self, Write};

use crate::{core::Core, instruction::*};

pub fn format(instr: Instr, core: Option<&Core>, addr: Option<u32>) -> String {
    let mut buff = Vec::new();
    PrettyPrint::new(&mut buff, core, addr)
        .pretty_print(instr)
        .unwrap();
    String::from_utf8(buff).unwrap()
}

pub struct PrettyPrint<'a, O> {
    _core: Option<&'a Core>,
    _addr: Option<u32>,
    out: O,
}
impl<'a, O: Write> PrettyPrint<'a, O> {
    pub fn new(out: O, _core: Option<&'a Core>, _addr: Option<u32>) -> Self {
        Self { _core, _addr, out }
    }

    pub fn pretty_print(&mut self, instr: Instr) -> io::Result<()> {
        let cc = match instr.cc {
            Cc::Equal => "EQ",
            Cc::NotEqual => "NE",
            Cc::UnsignedHigherOrSame => "CS",
            Cc::UnsignedLower => "CC",
            Cc::Negative => "MI",
            Cc::PositiveOrZero => "PL",
            Cc::Overflow => "VS",
            Cc::NoOverflow => "VC",
            Cc::UnsignedHigher => "HI",
            Cc::UnsignedLowerOrSame => "LS",
            Cc::GreaterOrEqual => "GE",
            Cc::LessThan => "LT",
            Cc::GreaterThan => "GT",
            Cc::LessThanOrEqual => "LE",
            Cc::Always => "",
            Cc::Reserved => unreachable!(),
        };

        #[allow(unused_variables)]
        match instr.op {
            Op::Bx(register) => {
                write!(self.out, "BX{cc} R{}", register.0)?;
            }
            Op::B(offset) => {
                write!(self.out, "B{cc} {offset}")?;
            }
            Op::Bl(offset) => write!(self.out, "BL{cc} {offset}")?,
            Op::Data {
                op: operation,
                s,
                rd,
                rn,
                op2: operand2,
            } => {
                self.print_data(cc, operation, s, rd, rn, operand2)?;
            }
            Op::Mrs { p, rd } => {
                let psr = if p { "SPSR" } else { "CPSR" };
                write!(self.out, "MRS R{}, {psr}", rd.0)?;
            }
            Op::Msr { p, fields, op2 } => {
                let psr = if p { "SPSR" } else { "CPSR" };
                let f = if fields.f { "F" } else { "" };
                let s = if fields.s { "S" } else { "" };
                let x = if fields.x { "X" } else { "" };
                let c = if fields.c { "C" } else { "" };

                write!(self.out, "MSR {psr}_{f}{s}{x}{c}, ")?;
                self.print_op2(op2)?;
            }
            Op::Mul { s, rd, rm, rs: rn } => {
                let s = if s { "S" } else { "" };
                write!(self.out, "MUL{cc}{s} R{}, R{}, R{}", rd.0, rm.0, rn.0)?;
            }
            Op::Mla { s, rd, rm, rs, rn } => {
                let s = if s { "S" } else { "" };
                write!(
                    self.out,
                    "MLA{cc}{s} R{}, R{}, R{}, R{}",
                    rd.0, rm.0, rs.0, rn.0
                )?;
            }
            Op::Mull {
                s,
                u,
                a,
                rd_lo,
                rd_hi,
                rm,
                rs,
            } => {
                let s = if s { "S" } else { "" };
                let name = if a { "MLAL" } else { "MULL" };
                let u = if u { "U" } else { "S" };
                write!(
                    self.out,
                    "{u}{name}{cc}{s} R{}, R{}, R{}, R{}",
                    rd_lo.0, rd_hi.0, rm.0, rs.0
                )?;
            }
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
            } => self.print_word_transfer(cc, p, u, b, w, l, rn, rd, offset)?,
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
            } => self.print_halfword_transfer(cc, p, u, w, l, s, h, rn, rd, offset)?,
            Op::BlockTransfer {
                p,
                u,
                s,
                w,
                l,
                rn,
                regs: register_list,
            } => self.print_block_transfer(cc, p, u, s, w, l, rn, register_list)?,
            Op::Swap { b, rn, rd, rm } => {
                let b = if b { "B" } else { "" };
                write!(self.out, "SWP{cc}{b} R{}, R{}, [R{}]", rd.0, rm.0, rn.0)?;
            }
            Op::Swi => write!(self.out, "SWI{cc}")?,
            Op::Undefined => write!(self.out, "UNDEFINED")?,

            Op::HiRegOps { op, rs, rd } => {
                let op = match op {
                    HiRegOp::Add => "ADD",
                    HiRegOp::Cmp => "CMP",
                    HiRegOp::Mov => "MOV",
                };

                write!(self.out, "{op} R{}, R{}", rd.0, rs.0)?;
            }
            Op::LoadAddr { sp, rd, offset } => {
                let offset = offset as u32 * 4;
                let sp = if sp { "SP " } else { "PC" };
                write!(self.out, "ADD R{}, {sp}, {offset:0>4x}", rd.0)?;
            }
            Op::BranchLong { h, offset } => {
                let h = if h { "1" } else { "0" };
                write!(self.out, "ThumbBranch{h} {offset}")?;
            }
            Op::LdrPc { rd, offset } => {
                write!(self.out, "LDR R{}, [PC + {:0>8x}]", rd.0, offset as u16 * 4)?;
            }
        }

        Ok(())
    }

    fn print_op2(&mut self, op2: Op2) -> io::Result<()> {
        match op2 {
            Op2::Register(shift) => self.print_shift(shift)?,
            Op2::Immediate(imm, shift) => write!(self.out, "{imm} ROR {shift}")?,
        }

        Ok(())
    }
    fn print_shift(&mut self, shift: Shift) -> io::Result<()> {
        let Shift {
            rm,
            shift_type,
            shift_count,
        } = shift;
        let shift = match shift_type {
            ShiftType::LogicalLeft => "LSL",
            ShiftType::LogicalRight => "LSR",
            ShiftType::ArithmeticRight => "ASR",
            ShiftType::RotateRight => "ROR",
        };
        write!(self.out, "R{} {shift} ", rm.0)?;

        match shift_count {
            ShiftCount::Constant(sa) => write!(self.out, "{sa}")?,
            ShiftCount::Register(register) => write!(self.out, "R{}", register.0)?,
        }

        Ok(())
    }

    fn print_data(
        &mut self,
        cc: &str,
        operation: DataOp,
        s: bool,
        rd: Reg,
        rn: Reg,
        op2: Op2,
    ) -> io::Result<()> {
        let s = if s { "S" } else { "" };
        let name = match operation {
            DataOp::And => "AND",
            DataOp::Eor => "EOR",
            DataOp::Sub => "SUB",
            DataOp::Rsb => "RSB",
            DataOp::Add => "ADD",
            DataOp::Adc => "ADC",
            DataOp::Sbc => "SBC",
            DataOp::Rsc => "RSC",
            DataOp::Tst => "TST",
            DataOp::Teq => "TEQ",
            DataOp::Cmp => "CMP",
            DataOp::Cmn => "CMN",
            DataOp::Orr => "ORR",
            DataOp::Mov => "MOV",
            DataOp::Bic => "BIC",
            DataOp::Mvn => "MVN",
        };

        write!(self.out, "{name}{cc}{s} ")?;

        if operation.has_result() {
            write!(self.out, "R{}, ", rd.0)?;
        }
        if !operation.is_unary() {
            write!(self.out, "R{}, ", rn.0)?;
        }

        self.print_op2(op2)?;

        Ok(())
    }
    fn print_word_transfer(
        &mut self,
        cc: &str,
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        l: bool,
        rn: Reg,
        rd: Reg,
        offset: WordOffset,
    ) -> io::Result<()> {
        let name = if l { "LDR" } else { "STR" };
        let b = if b { "B" } else { "" };
        write!(self.out, "{name}{cc}{b} R{}, ", rd.0)?;
        self.print_word_transfer_offset(p, u, w, rn, offset)?;

        Ok(())
    }
    fn print_word_transfer_offset(
        &mut self,
        p: bool,
        u: bool,
        w: bool,
        rn: Reg,
        offset: WordOffset,
    ) -> io::Result<()> {
        write!(self.out, "[ R{}", rn.0)?;
        if !p {
            write!(self.out, " ]")?;
        }

        let u = if u { "+" } else { "-" };
        write!(self.out, " {u} ")?;

        match offset {
            WordOffset::Immediate(offset) => write!(self.out, "{offset}")?,
            WordOffset::Shift(shift) => self.print_shift(shift)?,
        }

        if p {
            let w = if w { "!" } else { "" };
            write!(self.out, " ]{w}")?;
        }

        Ok(())
    }
    fn print_block_transfer(
        &mut self,
        cc: &str,
        p: bool,
        u: bool,
        s: bool,
        w: bool,
        l: bool,
        rn: Reg,
        register_list: u16,
    ) -> io::Result<()> {
        let dir = match (p, u) {
            (false, false) => "DA",
            (false, true) => "IA",
            (true, false) => "DB",
            (true, true) => "IB",
        };

        let name = if l { "LDM" } else { "STM" };
        let wb = if w { "!" } else { "" };
        write!(self.out, "{name}{cc}{dir} R{}{wb}, <", rn.0)?;

        let mut first = true;
        for i in 0..16 {
            let mask = 1 << i;
            if register_list & mask != 0 {
                if !first {
                    write!(self.out, ", ")?;
                }
                write!(self.out, "R{i}")?;
                first = false;
            }
        }

        let s = if s { "^" } else { "" };
        write!(self.out, ">{s}")?;

        Ok(())
    }

    fn print_halfword_transfer(
        &mut self,
        cc: &str,
        p: bool,
        u: bool,
        w: bool,
        l: bool,
        s: bool,
        h: bool,
        rn: Reg,
        rd: Reg,
        offset: HalfOffset,
    ) -> io::Result<()> {
        let name = if l { "LDR" } else { "STR" };
        let ty = match (s, h) {
            (false, false) => unreachable!(),
            (false, true) => "H",
            (true, false) => "SB",
            (true, true) => "SH",
        };

        write!(self.out, "{name}{ty}{cc} R{}, ", rd.0)?;
        self.print_halfword_transfer_offset(p, u, w, rn, offset)?;

        Ok(())
    }
    fn print_halfword_transfer_offset(
        &mut self,
        p: bool,
        u: bool,
        w: bool,
        rn: Reg,
        offset: HalfOffset,
    ) -> io::Result<()> {
        write!(self.out, "[ R{}", rn.0)?;
        if !p {
            write!(self.out, " ]")?;
        }

        let u = if u { "+" } else { "-" };
        write!(self.out, " {u} ")?;

        match offset {
            HalfOffset::Immediate(offset) => write!(self.out, "{offset}")?,
            HalfOffset::Register(rm) => write!(self.out, "R{}", rm.0)?,
        }

        if p {
            let w = if w { "!" } else { "" };
            write!(self.out, " ]{w}")?;
        }

        Ok(())
    }
}
