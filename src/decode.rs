use crate::instruction::{Condition, DataOperation, Instruction, Operand2, Operation, Register, Shift, ShiftCount, ShiftType, WordTransferOffset};


#[derive(Copy, Clone, Debug)]
pub struct ArmDecoder(u32);
impl ArmDecoder {
    pub fn new(instruction_word: u32) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instruction {
        let condition = self.decode_condition();

        let operation = match self.major() {
            0b000 | 0b001 => self.decode_major_zero_one(),
            0b011 if self.decode_flag(7) => Operation::Undefined,
            0b010 | 0b011 => self.decode_single_data_transfer(),
            0b100 => self.decode_block_data_transfer(),
            0b101 => self.decode_branch(),
            0b110 => self.decode_coprocessor_data_transfer(),
            8.. => unreachable!(),
            or => todo!("cannot decode major opcode {or:0>3b}"),
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
        if ((self.0 & 0x0FFFFFF0) >> 4) == 0b0001_0010_1111_1111_1111_0001 {
            self.decode_branch_and_exchange()
        }
        else if self.is_mrs() {
            self.decode_mrs()
        }
        else if self.is_msr() {
            self.decode_msr()
        }
        else if self.is_msr_flg() {
            self.decode_msr_flg()
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
        (self.0 >> 4) & 0x3FFFF == 0b101001111100000000
            && (self.0 >> 23) & 0x1F == 0b00010
    }
    fn is_msr_flg(&self) -> bool {
        (self.0 >> 4) & 0x3FFFF == 0b1010001111
            && ((self.0 >> 23) & 0x1F == 0b00010
                || (self.0 >> 23) & 0x1F == 0b00110)
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
        let rm = self.decode_register(0);
        let p = self.decode_flag(22);
        if p {
            Operation::MoveToSpsr(rm)
        }
        else {
            Operation::MoveToCpsr(rm)
        }
    }
    fn decode_msr_flg(&self) -> Operation {
        let src = self.decode_operand_2(self.decode_flag(25));
        let p = self.decode_flag(22);
        if p {
            Operation::MoveToSpsrf(src)
        }
        else {
            Operation::MoveToCpsrf(src)
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
            let imm = imm.rotate_right(shift * 2);
            Operand2::Immediate(imm as i32)
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
        todo!()
    }
}
