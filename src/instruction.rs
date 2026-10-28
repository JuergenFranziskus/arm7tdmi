use std::io::{self, Write};

pub mod print;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instr {
    pub cc: Cc,
    pub op: Op,
}
impl Instr {
    pub const NOP: Self = Self {
        cc: Cc::Always,
        op: Op::Nop,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cc {
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
impl Cc {
    pub fn pretty_print(self, out: &mut impl Write) -> io::Result<()> {
        use Cc::*;
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
pub enum Op {
    Bx(Reg),
    B(i32),
    Bl(i32),
    Data {
        op: DataOp,
        s: bool,
        rd: Reg,
        rn: Reg,
        op2: Op2,
    },
    Mrs {
        p: bool,
        rd: Reg,
    },
    Msr {
        p: bool,
        fields: PsrFields,
        op2: Op2,
    },
    Mul {
        s: bool,
        rd: Reg,
        rm: Reg,
        rs: Reg,
    },
    Mla {
        s: bool,
        rd: Reg,
        rm: Reg,
        rs: Reg,
        rn: Reg,
    },
    Mull {
        s: bool,
        u: bool,
        a: bool,
        rd_lo: Reg,
        rd_hi: Reg,
        rm: Reg,
        rs: Reg,
    },
    Nop,
    WordTransfer {
        p: bool,
        u: bool,
        b: bool,
        w: bool,
        l: bool,
        rn: Reg,
        rd: Reg,
        offset: WordOffset,
    },
    HalfwordTransfer {
        p: bool,
        u: bool,
        w: bool,
        l: bool,
        s: bool,
        h: bool,
        rn: Reg,
        rd: Reg,
        offset: HalfOffset,
    },
    BlockTransfer {
        p: bool,
        u: bool,
        s: bool,
        w: bool,
        l: bool,
        rn: Reg,
        regs: u16,
    },
    Swap {
        b: bool,
        rn: Reg,
        rd: Reg,
        rm: Reg,
    },
    Swi,
    Undefined,

    HiRegOps {
        op: HiRegOp,
        rs: Reg,
        rd: Reg,
    },
    LoadAddr {
        sp: bool,
        rd: Reg,
        offset: u8,
    },
    BranchLong {
        h: bool,
        offset: i32,
    },
    LdrPc {
        rd: Reg,
        offset: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataOp {
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
impl DataOp {
    pub fn has_result(self) -> bool {
        use DataOp::*;
        !matches!(self, Cmp | Cmn | Teq | Tst)
    }
    pub fn is_unary(self) -> bool {
        use DataOp::*;
        matches!(self, Mov | Mvn)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reg(pub u8);
impl Reg {
    pub fn new(code: u8) -> Self {
        assert!(code < 0x10);
        Self(code)
    }

    pub fn is_pc(self) -> bool {
        self == Self::PC
    }

    pub const STACK: Self = Self(13);
    pub const LINK: Self = Self(14);
    pub const PC: Self = Self(15);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op2 {
    Register(Shift),
    Immediate(u8, u8),
}
impl Op2 {
    pub fn has_register_shift(self) -> bool {
        matches!(
            self,
            Self::Register(Shift {
                shift_count: ShiftCount::Register(_),
                ..
            })
        )
    }
    pub fn is_register_shift(self, rs: Reg) -> bool {
        let Self::Register(shift) = self else {
            return false;
        };
        let ShiftCount::Register(reg) = shift.shift_count else {
            return false;
        };
        rs == reg
    }

    pub fn uses_reg(self, pc: Reg) -> bool {
        let Self::Register(shift) = self else {
            return false;
        };
        shift.rm == pc || shift.shift_count == ShiftCount::Register(pc)
    }

    pub fn register(reg: Reg) -> Op2 {
        Self::Register(Shift {
            rm: reg,
            shift_type: ShiftType::LogicalLeft,
            shift_count: ShiftCount::Constant(0),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shift {
    pub rm: Reg,
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
    Register(Reg),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordOffset {
    Immediate(u16),
    Shift(Shift),
}
impl WordOffset {
    pub fn uses_pc(self) -> bool {
        let Self::Shift(shift) = self else {
            return false;
        };
        shift.rm == Reg::PC || shift.shift_count == ShiftCount::Register(Reg::PC)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfOffset {
    Register(Reg),
    Immediate(u8),
}
impl HalfOffset {
    pub fn uses_pc(self) -> bool {
        let Self::Register(rm) = self else {
            return false;
        };
        rm == Reg::PC
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PsrFields {
    pub f: bool,
    pub s: bool,
    pub x: bool,
    pub c: bool,
}
impl PsrFields {
    pub(crate) fn mask(&self) -> u32 {
        let f = if self.f { 0xFF000000 } else { 0 };
        let s = if self.s { 0x00FF0000 } else { 0 };
        let x = if self.x { 0x0000FF00 } else { 0 };
        let c = if self.c { 0x000000FF } else { 0 };

        f | s | x | c
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiRegOp {
    Add,
    Cmp,
    Mov,
}
