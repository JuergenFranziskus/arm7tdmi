use std::io::{self, Write};

use crate::{core::Core, instruction::*};


pub fn format(instr: Instruction, core: Option<&Core>, addr: Option<u32>) -> String {
    let mut buff = Vec::new();
    PrettyPrint::new(&mut buff, core, addr).pretty_print(instr).unwrap();
    String::from_utf8(buff).unwrap()
}


pub struct PrettyPrint<'a, O> {
    core: Option<&'a Core>,
    addr: Option<u32>,
    out: O,    
}
impl<'a, O: Write> PrettyPrint<'a, O> {
    pub fn new(out: O, core: Option<&'a Core>, addr: Option<u32>) -> Self {
        Self {
            core,
            addr,
            out,
        }
    }

    pub fn pretty_print(&mut self, instr: Instruction) -> io::Result<()> {
        let cc = match instr.condition {
            Condition::Equal => "EQ",
            Condition::NotEqual => "NE",
            Condition::UnsignedHigherOrSame => "CS",
            Condition::UnsignedLower => "CC",
            Condition::Negative => "MI",
            Condition::PositiveOrZero => "PL",
            Condition::Overflow => "VS",
            Condition::NoOverflow => "VC",
            Condition::UnsignedHigher => "HI",
            Condition::UnsignedLowerOrSame => "LS",
            Condition::GreaterOrEqual => "GE",
            Condition::LessThan => "LT",
            Condition::GreaterThan => "GT",
            Condition::LessThanOrEqual => "LE",
            Condition::Always => "",
            Condition::Reserved => unreachable!(),
        };


        match instr.operation {
            Operation::Bx(register) => { write!(self.out, "BX{cc} R{}", register.0)?; }
            Operation::B(offset) => {
                write!(self.out, "B{cc} {offset}")?;
            }
            Operation::Bl(offset) => write!(self.out, "BL{cc} {offset}")?,
            Operation::Data { operation, s, rd, rn, operand2 } => { self.print_data(cc, operation, s, rd, rn, operand2)?; },
            Operation::MoveFromCpsr(register) => {
                write!(self.out, "MRS{cc} R{}, CPSR", register.0)?
            }
            Operation::MoveFromSpsr(register) => write!(self.out, "MRS{cc} R{}, SPSR", register.0)?,
            Operation::MoveToCpsr(fields, register) => {
                let x = if fields.x { "X" } else { "" };
                let f = if fields.f { "F" } else { "" };
                let s = if fields.s { "S" } else { "" };
                let c = if fields.c { "C" } else { "" };
                write!(self.out, "MSR{cc} CPSR_{f}{s}{x}{c}, ")?;
                self.print_op2(register)?;
            }
            Operation::MoveToSpsr(fields, register) => {
                let x = if fields.x { "X" } else { "" };
                let f = if fields.f { "F" } else { "" };
                let s = if fields.s { "S" } else { "" };
                let c = if fields.c { "C" } else { "" };
                write!(self.out, "MSR{cc} SPSR_{f}{s}{x}{c}, ")?;
                self.print_op2(register)?;
            }
            Operation::MoveToCpsrf(operand2) => todo!(),
            Operation::MoveToSpsrf(operand2) => todo!(),
            Operation::Mul { s, rd, rm, rs: rn } => {
                let s = if s { "S" } else { "" };
                write!(self.out, "MUL{cc}{s} R{}, R{}, R{}", rd.0, rm.0, rn.0)?;
            }
            Operation::Mla { s, rd, rm, rs, rn } => {
                let s = if s { "S" } else { "" };
                write!(self.out, "MLA{cc}{s} R{}, R{}, R{}, R{}", rd.0, rm.0, rs.0, rn.0)?;
            }
            Operation::Mull { s, u, a, rd_lo, rd_hi, rm, rs } => {
                let s = if s { "S" } else { "" };
                let name = if a { "MLAL" } else { "MULL" };
                let u = if u { "U" } else { "S" };
                write!(self.out, "{u}{name}{cc}{s} R{}, R{}, R{}, R{}", rd_lo.0, rd_hi.0, rm.0, rs.0)?;
            }
            Operation::Nop => todo!(),
            Operation::WordTransfer { p, u, b, w, l, rn, rd, offset } => self.print_word_transfer(cc, p, u, b, w, l, rn, rd, offset)?,
            Operation::HalfwordTransfer { p, u, w, l, s, h, rn, rd, offset } => self.print_halfword_transfer(cc, p, u, w, l, s, h, rn, rd, offset)?,
            Operation::BlockTransfer { p, u, s, w, l, rn, register_list } => self.print_block_transfer(cc, p, u, s, w, l, rn, register_list)?,
            Operation::Swap { b, rn, rd, rm } => {
                let b = if b { "B" } else { "" };
                write!(self.out, "SWP{cc}{b} R{}, R{}, [R{}]", rd.0, rm.0, rn.0)?;
            }
            Operation::Swi => write!(self.out, "SWI{cc}")?,
            Operation::CoprocessorData { cp_operation, crn, crd, cp_number, cp, crm } => todo!(),
            Operation::CoprocessorMemoryTransfer { p, u, n, w, l, rn, crd, cp_number, offset } => todo!(),
            Operation::CoprocessorRegisterTransfer { l, cp_opc, crn, rd, cp_number, cp, crm } => todo!(),
            Operation::Undefined => write!(self.out, "UNDEFINED")?,
        }

        Ok(())
    }

    fn print_op2(&mut self, op2: Operand2) -> io::Result<()> {
        match op2 {
            Operand2::Register(shift) => self.print_shift(shift)?,
            Operand2::Immediate(imm, shift) => write!(self.out, "{imm} ROR {shift}")?,
        }

        Ok(())
    }
    fn print_shift(&mut self, shift: Shift) -> io::Result<()> {
        let Shift { rm, shift_type, shift_count } = shift;
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


    fn print_data(&mut self, cc: &str, operation: DataOperation, s: bool, rd: Register, rn: Register, op2: Operand2) -> io::Result<()> {
        let s = if s { "S" } else { "" };
        let name = match operation {
            DataOperation::And => "AND",
            DataOperation::Eor => "EOR",
            DataOperation::Sub => "SUB",
            DataOperation::Rsb => "RSB",
            DataOperation::Add => "ADD",
            DataOperation::Adc => "ADC",
            DataOperation::Sbc => "SBC",
            DataOperation::Rsc => "RSC",
            DataOperation::Tst => "TST",
            DataOperation::Teq => "TEQ",
            DataOperation::Cmp => "CMP",
            DataOperation::Cmn => "CMN",
            DataOperation::Orr => "ORR",
            DataOperation::Mov => "MOV",
            DataOperation::Bic => "BIC",
            DataOperation::Mvn => "MVN",
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
    fn print_word_transfer(&mut self, cc: &str, p: bool, u: bool, b: bool, w: bool, l: bool, rn: Register, rd: Register, offset: WordTransferOffset) -> io::Result<()> {
        let name = if l { "LDR" } else { "STR" };
        let b = if b { "B" } else { "" };
        write!(self.out, "{name}{cc}{b} R{}, ", rd.0)?;
        self.print_word_transfer_offset(p, u, w, rn, offset)?;

        Ok(())
    }
    fn print_word_transfer_offset(&mut self, p: bool, u: bool, w: bool, rn: Register, offset: WordTransferOffset) -> io::Result<()> {
        write!(self.out, "[ R{}", rn.0)?;
        if !p {
            write!(self.out, " ]")?;
        }

        let u = if u { "+" } else { "-" };
        write!(self.out, " {u} ")?;

        match offset {
            WordTransferOffset::Immediate(offset) => write!(self.out, "{offset}")?,
            WordTransferOffset::Shift(shift) => self.print_shift(shift)?,
        }

        if p {
            let w = if w { "!" } else { "" };
            write!(self.out, " ]{w}")?;
        }

        Ok(())
    }
    fn print_block_transfer(&mut self, cc: &str, p: bool, u: bool, s: bool, w: bool, l: bool, rn: Register, register_list: u16) -> io::Result<()> {
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
    
    fn print_halfword_transfer(&mut self, cc: &str, p: bool, u: bool, w: bool, l: bool, s: bool, h: bool, rn: Register, rd: Register, offset: HalfwordTransferOffset) -> io::Result<()> {
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
    fn print_halfword_transfer_offset(&mut self, p: bool, u: bool, w: bool, rn: Register, offset: HalfwordTransferOffset) -> io::Result<()> {
        write!(self.out, "[ R{}", rn.0)?;
        if !p {
            write!(self.out, " ]")?;
        }

        let u = if u { "+" } else { "-" };
        write!(self.out, " {u} ")?;

        match offset {
            HalfwordTransferOffset::Immediate(offset) => write!(self.out, "{offset}")?,
            HalfwordTransferOffset::Register(rm) => write!(self.out, "R{}", rm.0)?,
        }

        if p {
            let w = if w { "!" } else { "" };
            write!(self.out, " ]{w}")?;
        }


        Ok(())
    }

}
