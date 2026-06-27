use crate::instruction::{Condition, CpsrFields, DataOperation, HalfwordTransferOffset, Instruction, Operand2, Operation, Register, Shift, ShiftCount, ShiftType, WordTransferOffset};


pub fn decode(word: u32, thumb: bool) -> Instruction {
    if thumb {
        ThumbDecoder::new(word as u16).decode()
    }
    else {
        ArmDecoder::new(word).decode()
    }
}


#[derive(Copy, Clone, Debug)]
pub struct ArmDecoder(u32);
impl ArmDecoder {
    pub fn new(instruction_word: u32) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instruction {
        let condition = self.decode_condition();
        let major = self.major();
        //println!("{major:0>3b}");
        let operation = match major {
            0b000 | 0b001 => self.decode_major_zero_one(),
            0b011 if self.decode_flag(4) => Operation::Undefined,
            0b010 | 0b011 => self.decode_single_data_transfer(),
            0b100 => self.decode_block_data_transfer(),
            0b101 => self.decode_branch(),
            0b110 => self.decode_coprocessor_data_transfer(),
            0b111 if self.decode_flag(24) => Operation::Swi,
            8.. => unreachable!(),
            or => Operation::Undefined,
        };

        Instruction {
            condition,
            operation,
        }
    }
    fn decode_condition(&self) -> Condition {
        let nibble = self.0 >> 28;
        match nibble {
            0 => Condition::Equal,
            1 => Condition::NotEqual,
            2 => Condition::UnsignedHigherOrSame,
            3 => Condition::UnsignedLower,
            4 => Condition::Negative,
            5 => Condition::PositiveOrZero,
            6 => Condition::Overflow,
            7 => Condition::NoOverflow,
            8 => Condition::UnsignedHigher,
            9 => Condition::UnsignedLowerOrSame,
            10 => Condition::GreaterOrEqual,
            11 => Condition::LessThan,
            12 => Condition::GreaterThan,
            13 => Condition::LessThanOrEqual,
            14 => Condition::Always,
            15 => Condition::Reserved,
            _ => unreachable!(),
        }
    }



    fn decode_major_zero_one(&self) -> Operation {
        if self.is_mul() {
            self.decode_mul()
        }
        else if self.is_mull() {
            self.decode_mull()
        }
        else if self.is_mrs() {
            self.decode_mrs()
        }
        else if self.is_msr() {
            self.decode_msr()
        }
        else if self.is_msr_f() {
            self.decode_msr_f()
        }
        else if ((self.0 & 0x0FFFFFF0) >> 4) == 0b0001_0010_1111_1111_1111_0001 {
            self.decode_branch_and_exchange()
        }
        
        else if self.is_halfword_transfer() {
            self.decode_half_transfer()
        }
        else {
            self.decode_data_operation()
        }
    }
    fn is_mrs(&self) -> bool {
        (self.0 & 0xFFF) == 0
            && (self.0 >> 16) & 0x3F == 0b001111
            && (self.0 >> 23) & 0x1F == 0b00010
    }
    fn is_msr(&self) -> bool {
        let i = self.decode_flag(25);
        if i {
            (self.0 >> 12) & 0xF == 0xF
                && (self.0 >> 20) & 0x1 == 0
                && (self.0 >> 23) & 0x3 == 0b10
                && (self.0 >> 26) & 0x3 == 0
        }
        else {
            (self.0 >> 4) & 0xFF == 0
                && (self.0 >> 20) & 0x1 == 0
                && (self.0 >> 23) & 0x3 == 0b10
                && (self.0 >> 26) & 0x3 == 0
        }
    }
    fn is_msr_f(&self) -> bool {
        (self.0 >> 4) & 0x3FFFF == 0b1010001111
            && ((self.0 >> 23) & 0x1F == 0b00010
                || (self.0 >> 23) & 0x1F == 0b00110)
    }
    fn is_halfword_transfer(&self) -> bool {
        let bit_7 = self.0 & 1 << 7 != 0;
        let bit_4 = self.0 & 1 << 4 != 0;
        self.major() == 0 && bit_7 && bit_4
    }
    fn is_mul(&self) -> bool {
        (self.0 >> 22) & 0x3F  == 0
            && (self.0 >> 4) & 0xF == 0b1001
    }
    fn is_mull(&self) -> bool {
        (self.0 >> 23) & 0x1F == 1
            && (self.0 >> 4) & 0xF == 0b1001
    }

    fn decode_branch_and_exchange(&self) -> Operation {
        let rn = self.decode_register(0);
        Operation::Bx(rn)
    }
    fn decode_mrs(&self) -> Operation {
        let rd = self.decode_register(12);
        let p = self.decode_flag(22);
        if p {
            Operation::MoveFromSpsr(rd)
        }
        else {
            Operation::MoveFromCpsr(rd)
        }
    }
    fn decode_msr(&self) -> Operation {
        let rm = self.decode_operand_2(self.decode_flag(25));
        let p = self.decode_flag(22);
        let f = self.decode_flag(19);
        let s = self.decode_flag(18);
        let x = self.decode_flag(17);
        let c = self.decode_flag(16);
        let flags = CpsrFields { f, s, x, c };
        if p {
            Operation::MoveToSpsr(flags, rm)
        }
        else {
            Operation::MoveToCpsr(flags, rm)
        }
    }
    fn decode_msr_f(&self) -> Operation {
        let src = self.decode_operand_2(self.decode_flag(25));
        let p = self.decode_flag(22);
        if p {
            Operation::MoveToSpsrf(src)
        }
        else {
            Operation::MoveToCpsrf(src)
        }
    }
    fn decode_mul(&self) -> Operation {
        let rm = self.decode_register(0);
        let rs = self.decode_register(8);
        let rn = self.decode_register(12);
        let rd = self.decode_register(16);
        let s = self.decode_flag(20);
        let a = self.decode_flag(21);

        if a {
            Operation::Mla { s, rd, rm, rs, rn }
        }
        else {
            Operation::Mul { s, rd, rm, rs }
        }
    }
    fn decode_half_transfer(&self) -> Operation {
        let p = self.decode_flag(24);
        let u = self.decode_flag(23);
        let i = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);
        let s = self.decode_flag(6);
        let h = self.decode_flag(5);

        let rn = self.decode_register(16);
        let rd = self.decode_register(12);
        let rm = self.decode_register(0);

        if !s && !h {
            Operation::Swap {
                b: i,
                rn,
                rd,
                rm,
            }
        }
        else {
            let offset = if i {
                let lo = self.0 & 0xF;
                let hi = (self.0 & 0xF00) >> 4;
                HalfwordTransferOffset::Immediate(lo as u8 | hi as u8)
            }
            else {
                HalfwordTransferOffset::Register(rm)
            };

            Operation::HalfwordTransfer { p, u, w, l, s, h, rn, rd, offset }
        }

    }
    fn decode_data_operation(&self) -> Operation {
        let i = self.decode_flag(25);
        let s = self.decode_flag(20);
        let opcode = (self.0 >> 21) as u8 & 0xF;
        let operation = match opcode {
            0 => DataOperation::And,
            1 => DataOperation::Eor,
            2 => DataOperation::Sub,
            3 => DataOperation::Rsb,
            4 => DataOperation::Add,
            5 => DataOperation::Adc,
            6 => DataOperation::Sbc,
            7 => DataOperation::Rsc,
            8 => DataOperation::Tst,
            9 => DataOperation::Teq,
            10 => DataOperation::Cmp,
            11 => DataOperation::Cmn,
            12 => DataOperation::Orr,
            13 => DataOperation::Mov,
            14 => DataOperation::Bic,
            15 => DataOperation::Mvn,
            16.. => unreachable!(),
        };

        let rn = self.decode_register(16);
        let rd = self.decode_register(12);
        let operand2 = self.decode_operand_2(i);
        Operation::Data {
            operation,
            s,
            rd,
            rn,
            operand2,
        }
    }
    fn decode_mull(&self) -> Operation {
        let rm = self.decode_register(0);
        let rs = self.decode_register(8);
        let rd_lo = self.decode_register(12);
        let rd_hi = self.decode_register(16);
        let s = self.decode_flag(20);
        let a = self.decode_flag(21);
        let u = self.decode_flag(22);

        Operation::Mull { s, u, a, rd_lo, rd_hi, rm, rs }
    }


    fn decode_single_data_transfer(&self) -> Operation {
        let i = self.decode_flag(25);
        let p = self.decode_flag(24);
        let u = self.decode_flag(23);
        let b = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);
        let rn = self.decode_register(16);
        let rd = self.decode_register(12);
        let offset = if i {
            WordTransferOffset::Shift(self.decode_shift())
        }
        else {
            WordTransferOffset::Immediate((self.0 & 0xFFF) as u16)
        };

        Operation::WordTransfer { p, u, b, w, l, rn, rd, offset }
    }
    fn decode_block_data_transfer(&self) -> Operation {
        let p = self.decode_flag(24);
        let u = self.decode_flag(23);
        let s = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);
        let rn = self.decode_register(16);
        let register_list = self.0 as u16;

        Operation::BlockTransfer { p, u, s, w, l, rn, register_list }
    }
    fn decode_branch(&self) -> Operation {
        let l = self.decode_flag(24);
        let offset = self.0 & 0xFFFFFF;
        let offset_sign = offset & (1 << 23) != 0;
        let offset_high = if offset_sign { 0xFF000000 } else { 0 };
        let offset = offset | offset_high;
        let offset = (offset as i32) << 2;
        if l {
            Operation::Bl(offset)
        }
        else {
            Operation::B(offset)
        }
    }
    fn decode_coprocessor_data_transfer(&self) -> Operation {
        let p = self.decode_flag(24);
        let u=  self.decode_flag(23);
        let n = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);

        let rn = self.decode_register(16);
        let crd = self.decode_register(12);
        let cp_number = (self.0 >> 8) as u8 & 0xF;
        let offset = self.0 as u8;

        Operation::CoprocessorMemoryTransfer {
            p,
            u,
            n,
            w,
            l,
            rn,
            crd,
            cp_number,
            offset,
        }
    }

    fn decode_operand_2(&self, i: bool) -> Operand2 {
        if i {
            let imm = self.0 & 0xFF;
            let shift = (self.0 >> 8) & 0xF;
            Operand2::Immediate(imm as u8, shift as u8)
        }
        else {
            Operand2::Register(self.decode_shift())
        }
    }

    fn decode_shift(&self) -> Shift {
        let rm = self.decode_register(0);
        let reg = self.decode_flag(4);
        let shift_type = (self.0 >> 5) & 0x3;
        let shift_type = match shift_type {
            0 => ShiftType::LogicalLeft,
            1 => ShiftType::LogicalRight,
            2 => ShiftType::ArithmeticRight,
            3 => ShiftType::RotateRight,
            4.. => unreachable!(),
        };

        if reg {
            let rs = self.decode_register(8);
            Shift {
                rm,
                shift_type,
                shift_count: ShiftCount::Register(rs),
            }
        }
        else {
            let amount = (self.0 >> 7) as u8 & 0x1F;
            Shift {
                rm,
                shift_type,
                shift_count: ShiftCount::Constant(amount),
            }
        }
    }

    fn decode_flag(&self, at: u32) -> bool {
        let mask = 1 << at;
        self.0 & mask != 0
    }
    fn decode_register(&self, at: u32) -> Register {
        let code = (self.0 >> at) as u8 & 0xF;
        Register::new(code)
    }
    fn major(&self) -> u8 {
        (self.0 >> 25) as u8 & 0x7
    }
}



#[derive(Copy, Clone, Debug)]
pub struct ThumbDecoder(u16);
impl ThumbDecoder {
    pub fn new(instruction_word: u16) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instruction {
        Instruction { condition: Condition::Always, operation: Operation::Undefined }
    }
}
