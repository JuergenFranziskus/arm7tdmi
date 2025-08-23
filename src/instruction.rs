use std::io::{self, Write};



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub condition: Condition,
    pub operation: Operation,
}
impl Instruction {
    pub fn pretty_print(self, out: &mut impl Write, address: Option<u32>) -> io::Result<()> {
        self.operation.pretty_print(out, self.condition, address)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Equal,
    NotEqual,
    UnsignedHigherOrSame,
    UnsignedLower,
    Negative,
    PositiveOrZero,
    Overflow,
    NoOverflow,
    UnsignedHigher,
    UnsignedLowerOrSame,
    GreaterOrEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    Always,
    Reserved,
}
impl Condition {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        use Condition::*;
        match self {
            Equal => write!(out, "EQ")?,
            NotEqual => write!(out, "NE")?,
            UnsignedHigherOrSame => write!(out, "CS")?,
            UnsignedLower => write!(out, "CC")?,
            Negative => write!(out, "MI")?,
            PositiveOrZero => write!(out, "PL")?,
            Overflow => write!(out, "VS")?,
            NoOverflow => write!(out, "VC")?,
            UnsignedHigher => write!(out, "HI")?,
            UnsignedLowerOrSame => write!(out, "LS")?,
            GreaterOrEqual => write!(out, "GE")?,
            LessThan => write!(out, "LT")?,
            GreaterThan => write!(out, "GT")?,
            LessThanOrEqual => write!(out, "LE")?,
            Always => (),
            Reserved => write!(out, "RESERVED_CONDITION!!")?,
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Bx(Register),
    B(i32),
    Bl(i32),
    Data {
        operation: DataOperation,
        s: bool,
        rd: Register,
        rn: Register,
        operand2: Operand2,
    },
    MoveFromCpsr(Register),
    MoveFromSpsr(Register),
    MoveToCpsr(Register),
    MoveToSpsr(Register),
    MoveToCpsrf(Operand2),
    MoveToSpsrf(Operand2),
    Mul {
        s: bool,
        rd: Register,
        rm: Register,
        rn: Register,
    },
    Mla {
        s: bool,
        rd: Register,
        rm: Register,
        rs: Register,
        rn: Register,
    },
    Mull {
        s: bool,
        u: bool,
        a: bool,
        rd_lo: Register,
        rd_hi: Register,
        rm: Register,
        rs: Register,
    },
    WordTransfer {
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        l: bool,
        rn: Register,
        rd: Register,
        offset: WordTransferOffset,
    },
    HalfwordTransfer {
        p: bool,
        u: bool,
        w: bool,
        l: bool,
        s: bool,
        h: bool,
        rn: Register,
        rd: Register,
        offset: HalfwordTransferOffset,
    },
    BlockTransfer {
        p: bool,
        u: bool,
        s: bool,
        w: bool,
        l: bool,
        rn: Register,
        register_list: u16,
    },
    Swap {
        b: bool,
        rn: Register,
        rd: Register,
        rm: Register,
    },
    Swi,
    CoprocessorData {
        cp_operation: u8,
        crn: Register,
        crd: Register,
        cp_number: u8,
        cp: u8,
        crm: Register,
    },
    CoprocessorMemoryTransfer {
        p: bool,
        u: bool,
        n: bool,
        w: bool,
        l: bool,
        rn: Register,
        crd: Register,
        cp_number: u8,
        offset: u8,
    },
    CoprocessorRegisterTransfer {
        l: bool,
        cp_opc: u8,
        crn: Register,
        rd: Register,
        cp_number: u8,
        cp: u8,
        crm: Register,
    },
    Undefined,
}
impl Operation {
    pub fn pretty_print(self, out: &mut impl Write, condition: Condition, address: Option<u32>) -> io::Result<()> {
        match self {
            Operation::Bx(rn) => {
                write!(out, "BX")?;
                condition.pretty_print(out)?;
                write!(out, " ")?;
                rn.pretty_print(out)?;
            }
            Operation::B(offset) => {
                write!(out, "B")?;
                condition.pretty_print(out)?;
                let sign = if offset < 0 { "-" } else { "" };
                let abs = offset.abs();
                write!(out, " {sign}{abs:x}")?;

                if let Some(address) = address {
                    let target = address.wrapping_add_signed(offset).wrapping_add(8);
                    write!(out, " => {target:0>8x}")?;
                }
            }
            Operation::Bl(offset) => {
                write!(out, "BL")?;
                condition.pretty_print(out)?;
                let sign = if offset < 0 { "-" } else { "" };
                let abs = offset.abs();
                write!(out, " {sign}{abs:x}")?;

                if let Some(address) = address {
                    let target = address.wrapping_add_signed(offset).wrapping_add(8);
                    write!(out, " => {target:0>8x}")?;
                }
            }
            Operation::Data { operation, s, rd, rn, operand2 } => {
                operation.pretty_print(out)?;
                condition.pretty_print(out)?;
                if s { write!(out, "S")?; }
                write!(out, " ")?;
                if operation.has_result() {
                    rd.pretty_print(out)?;
                    write!(out, ", ")?;
                }
                if !operation.is_unary() {
                    rn.pretty_print(out)?;
                    write!(out, ", ")?;
                }
                operand2.pretty_print(out)?;
            }
            Operation::MoveFromCpsr(rd) => {
                write!(out, "MRS")?;
                condition.pretty_print(out)?;
                write!(out, " ")?;
                rd.pretty_print(out)?;
                write!(out, ", CPSR")?;
            }
            Operation::MoveFromSpsr(rd) => {
                write!(out, "MRS")?;
                condition.pretty_print(out)?;
                write!(out, " ")?;
                rd.pretty_print(out)?;
                write!(out, ", SPSR")?;
            }
            Operation::MoveToCpsr(rm) => {
                write!(out, "MSR")?;
                condition.pretty_print(out)?;
                write!(out, " CPSR, ")?;
                rm.pretty_print(out)?;
            }
            Operation::MoveToSpsr(rm) => {
                write!(out, "MSR")?;
                condition.pretty_print(out)?;
                write!(out, " SPSR, ")?;
                rm.pretty_print(out)?;
            }
            Operation::MoveToCpsrf(rm) => {
                write!(out, "MSR")?;
                condition.pretty_print(out)?;
                write!(out, " CPSR_flg, ")?;
                rm.pretty_print(out)?;
            }
            Operation::MoveToSpsrf(rm) => {
                write!(out, "MSR")?;
                condition.pretty_print(out)?;
                write!(out, " SPSR_flg, ")?;
                rm.pretty_print(out)?;
            }
            Operation::WordTransfer { p, u, b, w, l, rn, rd, offset } => {
                let name = if l { "LDR" } else { "STR" };
                let byte_suffix = if b { "B" } else { "" };
                let force_user_mode = if !p && w { "T" } else { "" };
                let sign = if u { "+" } else {"-" };
                write!(out, "{name}")?;
                condition.pretty_print(out)?;
                write!(out, "{byte_suffix}{force_user_mode} ")?;
                rd.pretty_print(out)?;
                write!(out, ", [")?;
                rn.pretty_print(out)?;

                if !p {
                    write!(out, "], ")?;
                }
                else {
                    write!(out, " ")?;
                }
                write!(out, "{sign} ")?;
                offset.pretty_print(out)?;

                if p {
                    write!(out, "]")?;
                    if w {
                        write!(out, "!")?;
                    }
                }
            }
            Operation::BlockTransfer { p, u, s, w, l, rn, mut register_list } => {
                let name = if l { "LDM" } else { "STM" } ;
                let suffix = match (p, u) {
                    (true, true) => "IB",
                    (false, true) => "IA",
                    (true, false) => "DB",
                    (false, false) => "DA",
                };
                let write_back = if w { "!" } else { "" };
                let caret = if s { "^" } else { "" };

                write!(out, "{name}{suffix} ")?;
                rn.pretty_print(out)?;
                write!(out, "{write_back}, {{")?;
                for i in 0..16 {
                    let mask = 1 << i;
                    if register_list & mask == 0 { continue };
                    Register::new(i).pretty_print(out)?;
                    register_list &= !mask;
                    if register_list != 0 {
                        write!(out, ", ")?;
                    }
                }
                write!(out, "}}{caret}")?;

            }
            Operation::Undefined => write!(out, "UNDEFINED")?,
            Operation::CoprocessorMemoryTransfer { p, u, n, w, l, rn, crd, cp_number, offset } => {
                let name = if l { "LDC" } else { "STC" };
                let long = if n { "L" } else { "" };
                write!(out, "{name}")?;
                condition.pretty_print(out)?;
                write!(out, "{long} {cp_number}#, c")?;
                crd.pretty_print(out)?;
                write!(out, ", [")?;

                rn.pretty_print(out)?;

                if !p {
                    write!(out, "], ")?;
                }
                else {
                    write!(out, " ")?;
                }
                let sign = if u { "+" } else {"-" };
                write!(out, "{sign} {offset:x}")?;

                if p {
                    write!(out, "]")?;
                    if w {
                        write!(out, "!")?;
                    }
                }
            }
            _ => write!(out, "UNPRINTABLE")?,
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataOperation {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}
impl DataOperation {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        use DataOperation::*;
        let print = match self {
            And => "AND",
            Eor => "EOR",
            Sub => "SUB",
            Rsb => "RSB",
            Add => "ADD",
            Adc => "ADC",
            Sbc => "SBC",
            Rsc => "RSC",
            Tst => "TST",
            Teq => "TEQ",
            Cmp => "CMP",
            Cmn => "CMN",
            Orr => "ORR",
            Mov => "MOV",
            Bic => "BIC",
            Mvn => "MVN",
        };
        write!(out, "{print}")?;
        Ok(())
    }

    pub fn has_result(self) -> bool {
        use DataOperation::*;
        !matches!(self, Cmp | Cmn | Teq | Tst)
    }
    pub fn is_unary(self) -> bool {
        use DataOperation::*;
        matches!(self, Mov | Mvn)
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register(u8);
impl Register {
    pub fn new(code: u8) -> Self {
        assert!(code < 0x10);
        Self(code)
    }

    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "r{}", self.0)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand2 {
    Register(Shift),
    Immediate(i32),
}
impl Operand2 {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Register(shift) => shift.pretty_print(out)?,
            Self::Immediate(value) => {
                let sign = if value < 0 { "-" } else { "" };
                let abs = value.abs();
                write!(out, "#{sign}{abs:x}")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shift {
    pub rm: Register,
    pub shift_type: ShiftType,
    pub shift_count: ShiftCount,

}
impl Shift {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "UNPRINTABLE_SHIFT")?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftType {
    LogicalLeft,
    LogicalRight,
    ArithmeticRight,
    RotateRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftCount {
    Constant(u8),
    Register(Register),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordTransferOffset {
    Immediate(u16),
    Shift(Shift),
}
impl WordTransferOffset {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Immediate(offset) => write!(out, "#{offset:0>4x}")?,
            Self::Shift(shift) => shift.pretty_print(out)?,
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfwordTransferOffset {
    Register(Register),
    Immediate(u8),
}
