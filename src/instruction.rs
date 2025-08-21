

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
    MoveToCpsr(Field, Register, Operand2),
    MoveToSpsr(Field, Register, Operand2),
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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field(u8);
impl Field {
    const F: u8 = 1 << 0;
    const S: u8 = 1 << 1;
    const X: u8 = 1 << 2;
    const C: u8 = 1 << 3;

    pub fn f(self) -> bool { self.0 & Self::F != 0 }
    pub fn s(self) -> bool { self.0 & Self::S != 0 }
    pub fn x(self) -> bool { self.0 & Self::X != 0 }
    pub fn c(self) -> bool { self.0 & Self::C != 0 }

    pub fn set_f(&mut self, to: bool) { self.0 &= !Self::F; if to { self.0 |= Self::F; }}
    pub fn set_s(&mut self, to: bool) { self.0 &= !Self::S; if to { self.0 |= Self::S; }}
    pub fn set_x(&mut self, to: bool) { self.0 &= !Self::X; if to { self.0 |= Self::X; }}
    pub fn set_c(&mut self, to: bool) { self.0 &= !Self::C; if to { self.0 |= Self::C; }}

    pub fn with_f(mut self, to: bool) -> Self { self.set_f(to); self }
    pub fn with_s(mut self, to: bool) -> Self { self.set_s(to); self }
    pub fn with_x(mut self, to: bool) -> Self { self.set_x(to); self }
    pub fn with_c(mut self, to: bool) -> Self { self.set_c(to); self }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register(u8);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand2 {
    Register(Shift),
    Immediate(u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shift {
    pub rm: Register,
    pub shift_type: ShiftType,
    pub shift_count: Operand2ShiftCount,

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftType {
    LogicalLeft,
    LogicalRight,
    ArithmeticRight,
    RotateRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand2ShiftCount {
    Constant(u8),
    Register(Register),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordTransferOffset {
    Immediate(u16),
    Shift(Shift),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfwordTransferOffset {
    Register(Register),
    Immediate(u8),
}
