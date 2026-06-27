use std::io::{self, Write};

pub mod print;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub condition: Condition,
    pub operation: Operation,
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
    MoveToCpsr(CpsrFields, Operand2),
    MoveToSpsr(CpsrFields, Operand2),
    MoveToCpsrf(Operand2),
    MoveToSpsrf(Operand2),
    Mul {
        s: bool,
        rd: Register,
        rm: Register,
        rs: Register,
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
    Nop,
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
pub struct Register(pub u8);
impl Register {
    pub fn new(code: u8) -> Self {
        assert!(code < 0x10);
        Self(code)
    }

    pub const LINK: Self = Self(14);
    pub const PC: Self = Self(15);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand2 {
    Register(Shift),
    Immediate(u8, u8),
}
impl Operand2 {
    pub fn has_register_shift(self) -> bool {
        matches!(self, Self::Register(Shift { shift_count: ShiftCount::Register(_), .. }))
    }
    pub fn is_register_shift(self, rs: Register) -> bool {
        let Self::Register(shift) = self else { return false };
        let ShiftCount::Register(reg) = shift.shift_count else { return false };
        rs == reg
    }
    
    pub fn uses_reg(self, pc: Register) -> bool {
        let Self::Register(shift) = self else { return false };
        shift.rm == pc || shift.shift_count == ShiftCount::Register(pc)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shift {
    pub rm: Register,
    pub shift_type: ShiftType,
    pub shift_count: ShiftCount,

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
    pub fn uses_pc(self) -> bool {
        let Self::Shift(shift) = self else { return false };
        shift.rm == Register::PC || shift.shift_count == ShiftCount::Register(Register::PC)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfwordTransferOffset {
    Register(Register),
    Immediate(u8),
}
impl HalfwordTransferOffset {
    pub fn uses_pc(self) -> bool {
        let Self::Register(rm) = self else { return false };
        rm == Register::PC
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpsrFields {
    pub c: bool,
    pub f: bool,
    pub s: bool,
    pub x: bool,
}
impl CpsrFields {
    pub fn mask(self, user: bool) -> u32 {
        let c = if self.c { 0xFF } else { 0 };
        let x = if self.x { 0xFF00 } else { 0 };
        let s = if self.s { 0xFF0000 } else { 0 };
        let f = if self.f { 0xFF000000 } else { 0 };
        if user {
            f
        }
        else {
            c | f | x | s
        }
    }
}
