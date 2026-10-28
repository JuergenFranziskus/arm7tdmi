use crate::instruction::{
    Cc, DataOp, HalfOffset, HiRegOp, Instr, Op, Op2, PsrFields, Reg, Shift, ShiftCount, ShiftType,
    WordOffset,
};

pub fn decode(word: u32, thumb: bool) -> Instr {
    if thumb {
        ThumbDecoder::new(word as u16).decode()
    } else {
        ArmDecoder::new(word).decode()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ArmDecoder(u32);
impl ArmDecoder {
    pub fn new(instruction_word: u32) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instr {
        let condition = self.decode_condition();
        let major = self.major();
        //println!("{major:0>3b}");
        let operation = match major {
            0b000 | 0b001 => self.decode_major_zero_one(),
            0b011 if self.decode_flag(4) => Op::Undefined,
            0b010 | 0b011 => self.decode_single_data_transfer(),
            0b100 => self.decode_block_data_transfer(),
            0b101 => self.decode_branch(),
            0b111 if self.decode_flag(24) => Op::Swi,
            8.. => unreachable!(),
            _ => Op::Undefined,
        };

        Instr {
            cc: condition,
            op: operation,
        }
    }
    fn decode_condition(&self) -> Cc {
        let nibble = self.0 >> 28;
        get_cc(nibble as u8)
    }

    fn decode_major_zero_one(&self) -> Op {
        if self.is_mul() {
            self.decode_mul()
        } else if self.is_mull() {
            self.decode_mull()
        } else if ((self.0 & 0x0FFFFFF0) >> 4) == 0b0001_0010_1111_1111_1111_0001 {
            self.decode_branch_and_exchange()
        } else if self.is_halfword_transfer() {
            self.decode_half_transfer()
        } else {
            self.decode_data_operation()
        }
    }
    fn is_halfword_transfer(&self) -> bool {
        let bit_7 = self.0 & 1 << 7 != 0;
        let bit_4 = self.0 & 1 << 4 != 0;
        self.major() == 0 && bit_7 && bit_4
    }
    fn is_mul(&self) -> bool {
        (self.0 >> 22) & 0x3F == 0 && (self.0 >> 4) & 0xF == 0b1001
    }
    fn is_mull(&self) -> bool {
        (self.0 >> 23) & 0x1F == 1 && (self.0 >> 4) & 0xF == 0b1001
    }

    fn decode_branch_and_exchange(&self) -> Op {
        let rn = self.decode_register(0);
        Op::Bx(rn)
    }
    fn decode_mul(&self) -> Op {
        let rm = self.decode_register(0);
        let rs = self.decode_register(8);
        let rn = self.decode_register(12);
        let rd = self.decode_register(16);
        let s = self.decode_flag(20);
        let a = self.decode_flag(21);

        if a {
            Op::Mla { s, rd, rm, rs, rn }
        } else {
            Op::Mul { s, rd, rm, rs }
        }
    }
    fn decode_half_transfer(&self) -> Op {
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
            Op::Swap { b: i, rn, rd, rm }
        } else {
            let offset = if i {
                let lo = self.0 & 0xF;
                let hi = (self.0 & 0xF00) >> 4;
                HalfOffset::Immediate(lo as u8 | hi as u8)
            } else {
                HalfOffset::Register(rm)
            };

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
            }
        }
    }
    fn decode_data_operation(&self) -> Op {
        let i = self.decode_flag(25);
        let s = self.decode_flag(20);
        let opcode = (self.0 >> 21) as u8 & 0xF;
        let operation = match opcode {
            0 => DataOp::And,
            1 => DataOp::Eor,
            2 => DataOp::Sub,
            3 => DataOp::Rsb,
            4 => DataOp::Add,
            5 => DataOp::Adc,
            6 => DataOp::Sbc,
            7 => DataOp::Rsc,
            8 => DataOp::Tst,
            9 => DataOp::Teq,
            10 => DataOp::Cmp,
            11 => DataOp::Cmn,
            12 => DataOp::Orr,
            13 => DataOp::Mov,
            14 => DataOp::Bic,
            15 => DataOp::Mvn,
            16.. => unreachable!(),
        };

        if !operation.has_result() && !s {
            self.decode_msr_mrs()
        } else {
            let rn = self.decode_register(16);
            let rd = self.decode_register(12);
            let operand2 = self.decode_op2(i);
            Op::Data {
                op: operation,
                s,
                rd,
                rn,
                op2: operand2,
            }
        }
    }
    fn decode_msr_mrs(&self) -> Op {
        let opcode = self.decode_flag(21);
        let p = self.decode_flag(22);
        let rd = self.decode_register(12);

        if opcode {
            let i = self.decode_flag(25);
            let f = self.decode_flag(19);
            let s = self.decode_flag(18);
            let x = self.decode_flag(17);
            let c = self.decode_flag(16);

            let fields = PsrFields { f, s, x, c };
            let op2 = self.decode_op2(i);

            Op::Msr { p, fields, op2 }
        } else {
            Op::Mrs { p, rd }
        }
    }
    fn decode_mull(&self) -> Op {
        let rm = self.decode_register(0);
        let rs = self.decode_register(8);
        let rd_lo = self.decode_register(12);
        let rd_hi = self.decode_register(16);
        let s = self.decode_flag(20);
        let a = self.decode_flag(21);
        let u = self.decode_flag(22);

        Op::Mull {
            s,
            u,
            a,
            rd_lo,
            rd_hi,
            rm,
            rs,
        }
    }

    fn decode_single_data_transfer(&self) -> Op {
        let i = self.decode_flag(25);
        let p = self.decode_flag(24);
        let u = self.decode_flag(23);
        let b = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);
        let rn = self.decode_register(16);
        let rd = self.decode_register(12);
        let offset = if i {
            WordOffset::Shift(self.decode_shift())
        } else {
            WordOffset::Immediate((self.0 & 0xFFF) as u16)
        };

        Op::WordTransfer {
            p,
            u,
            b,
            w,
            l,
            rn,
            rd,
            offset,
        }
    }
    fn decode_block_data_transfer(&self) -> Op {
        let p = self.decode_flag(24);
        let u = self.decode_flag(23);
        let s = self.decode_flag(22);
        let w = self.decode_flag(21);
        let l = self.decode_flag(20);
        let rn = self.decode_register(16);
        let register_list = self.0 as u16;

        Op::BlockTransfer {
            p,
            u,
            s,
            w,
            l,
            rn,
            regs: register_list,
        }
    }
    fn decode_branch(&self) -> Op {
        let l = self.decode_flag(24);
        let offset = self.0 & 0xFFFFFF;
        let offset_sign = offset & (1 << 23) != 0;
        let offset_high = if offset_sign { 0xFF000000 } else { 0 };
        let offset = offset | offset_high;
        let offset = (offset as i32) << 2;
        if l { Op::Bl(offset) } else { Op::B(offset) }
    }

    fn decode_op2(&self, i: bool) -> Op2 {
        if i {
            let imm = self.0 & 0xFF;
            let shift = (self.0 >> 8) & 0xF;
            Op2::Immediate(imm as u8, shift as u8 * 2)
        } else {
            Op2::Register(self.decode_shift())
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
        } else {
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
    fn decode_register(&self, at: u32) -> Reg {
        let code = (self.0 >> at) as u8 & 0xF;
        Reg::new(code)
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

    fn get_flag(self, i: u16) -> bool {
        (self.0 & 1 << i) != 0
    }
    fn get_reg(self, at: u16, h: bool) -> Reg {
        let lo = (self.0 >> at) as u8 & 0x7;
        let hi = if h { 0b1000 } else { 0 };
        Reg(lo | hi)
    }

    pub fn decode(self) -> Instr {
        let primary = self.0 >> 13;
        match primary {
            0b000 => self.decode_000(),
            0b001 => self.decode_alu_imm(),
            0b010 => self.decode_010(),
            0b011 => self.decode_ldr_str_imm(),
            0b100 => self.decode_100(),
            0b101 => self.decode_101(),
            0b110 => self.decode_110(),
            0b111 => self.decode_111(),
            8.. => unreachable!(),
        }
    }

    fn decode_000(self) -> Instr {
        let bit = self.get_flag(12);

        if bit {
            self.decode_000_1()
        } else {
            self.decode_shift()
        }
    }
    fn decode_000_1(self) -> Instr {
        let bit = self.get_flag(11);

        if bit {
            self.decode_add_sub()
        } else {
            self.decode_shift()
        }
    }

    fn decode_010(self) -> Instr {
        let bit = self.get_flag(12);

        if bit {
            self.decode_010_1()
        } else {
            self.decode_010_0()
        }
    }
    fn decode_010_0(self) -> Instr {
        let bit = self.get_flag(11);

        if bit {
            self.decode_pc_rel_load()
        } else {
            self.decode_010_0_0()
        }
    }
    fn decode_010_0_0(self) -> Instr {
        let bit = self.get_flag(10);

        if bit {
            self.decode_hi_reg_ops()
        } else {
            self.decode_data_op()
        }
    }
    fn decode_010_1(self) -> Instr {
        if self.get_flag(9) {
            self.decode_load_store_signed_half()
        } else {
            self.decode_ldr_str_reg()
        }
    }

    fn decode_100(self) -> Instr {
        if self.get_flag(12) {
            self.decode_sp_rel_ldr_str()
        } else {
            self.decode_load_store_half()
        }
    }

    fn decode_101(self) -> Instr {
        let bit = self.get_flag(12);

        if bit {
            self.decode_101_1()
        } else {
            self.decode_load_addr()
        }
    }
    fn decode_101_1(self) -> Instr {
        if self.get_flag(10) {
            self.decode_push_pop()
        } else {
            self.decode_add_sub_sp()
        }
    }

    fn decode_110(self) -> Instr {
        if self.get_flag(12) {
            self.decode_cc_branch()
        } else {
            self.decode_ldm_stm()
        }
    }

    fn decode_111(self) -> Instr {
        if self.get_flag(12) {
            self.decode_branch_long()
        } else {
            self.decode_branch()
        }
    }

    fn decode_add_sub(self) -> Instr {
        let i = self.get_flag(10);
        let op = self.get_flag(9);
        let rd = self.get_reg(0, false);
        let rs = self.get_reg(3, false);
        let rn = self.get_reg(6, false);

        let op = if !op { DataOp::Add } else { DataOp::Sub };

        let op = if i {
            Op::Data {
                op,
                s: true,
                rd,
                rn: rs,
                op2: Op2::Immediate(rn.0, 0),
            }
        } else {
            Op::Data {
                op,
                s: true,
                rd,
                rn: rs,
                op2: Op2::register(rn),
            }
        };

        Instr { cc: Cc::Always, op }
    }
    fn decode_hi_reg_ops(self) -> Instr {
        let h1 = self.get_flag(7);
        let h2 = self.get_flag(6);
        let rs = self.get_reg(3, h2);
        let rd = self.get_reg(0, h1);
        let op = (self.0 >> 8) & 3;

        let op = match op {
            0 => Op::HiRegOps {
                op: HiRegOp::Add,
                rs,
                rd,
            },
            1 => Op::HiRegOps {
                op: HiRegOp::Cmp,
                rs,
                rd,
            },
            2 => Op::HiRegOps {
                op: HiRegOp::Mov,
                rs,
                rd,
            },
            3 => Op::Bx(rs),
            4.. => unreachable!(),
        };

        Instr { cc: Cc::Always, op }
    }
    fn decode_load_addr(self) -> Instr {
        let sp = self.get_flag(11);
        let rd = self.get_reg(8, false);
        let offset = self.0 as u8;

        Instr {
            cc: Cc::Always,
            op: Op::LoadAddr { sp, rd, offset },
        }
    }
    fn decode_add_sub_sp(self) -> Instr {
        let s = self.get_flag(7);
        let offset = self.0 as u8 & 0x7F;

        let op = if s { DataOp::Sub } else { DataOp::Add };

        Instr {
            cc: Cc::Always,
            op: Op::Data {
                op,
                s: false,
                rd: Reg::STACK,
                rn: Reg::STACK,
                op2: Op2::Immediate(offset, 30),
            },
        }
    }
    fn decode_branch(self) -> Instr {
        let offset = self.0 as i32 & 0x7FF;
        let sign = offset & (1 << 10) != 0;
        let mask = if sign { 0xFFFFF800u32.cast_signed() } else { 0 };
        let offset = (offset | mask) * 2;

        Instr {
            cc: Cc::Always,
            op: Op::B(offset),
        }
    }
    fn decode_cc_branch(self) -> Instr {
        let offset = self.0 as i8 as i32 * 2;
        let cc = (self.0 >> 8) as u8 & 0xF;

        if cc == 0xF {
            Instr {
                cc: Cc::Always,
                op: Op::Swi,
            }
        } else {
            Instr {
                cc: get_cc(cc),
                op: Op::B(offset),
            }
        }
    }
    fn decode_branch_long(self) -> Instr {
        let h = self.get_flag(11);
        let offset = self.0 as i32 & 0x7FF;
        let sign = offset & (1 << 10) != 0;
        let mask = if sign { 0xFFFFF800u32.cast_signed() } else { 0 };
        let offset = offset | mask;

        Instr {
            cc: Cc::Always,
            op: Op::BranchLong { h, offset },
        }
    }
    fn decode_data_op(self) -> Instr {
        let rs = self.get_reg(3, false);
        let rd = self.get_reg(0, false);
        let op = (self.0 >> 6) & 0xF;

        let op = match op {
            0 => Op::Data {
                op: DataOp::And,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            1 => Op::Data {
                op: DataOp::Eor,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            2 => Op::Data {
                op: DataOp::Mov,
                s: true,
                rd,
                rn: rd,
                op2: Op2::Register(Shift {
                    rm: rd,
                    shift_type: ShiftType::LogicalLeft,
                    shift_count: ShiftCount::Register(rs),
                }),
            },
            3 => Op::Data {
                op: DataOp::Mov,
                s: true,
                rd,
                rn: rd,
                op2: Op2::Register(Shift {
                    rm: rd,
                    shift_type: ShiftType::LogicalRight,
                    shift_count: ShiftCount::Register(rs),
                }),
            },
            4 => Op::Data {
                op: DataOp::Mov,
                s: true,
                rd,
                rn: rd,
                op2: Op2::Register(Shift {
                    rm: rd,
                    shift_type: ShiftType::ArithmeticRight,
                    shift_count: ShiftCount::Register(rs),
                }),
            },
            5 => Op::Data {
                op: DataOp::Adc,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            6 => Op::Data {
                op: DataOp::Sbc,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            7 => Op::Data {
                op: DataOp::Mov,
                s: true,
                rd,
                rn: rd,
                op2: Op2::Register(Shift {
                    rm: rd,
                    shift_type: ShiftType::RotateRight,
                    shift_count: ShiftCount::Register(rs),
                }),
            },
            8 => Op::Data {
                op: DataOp::Tst,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            9 => Op::Data {
                op: DataOp::Rsb,
                s: true,
                rd,
                rn: rs,
                op2: Op2::Immediate(0, 0),
            },
            10 => Op::Data {
                op: DataOp::Cmp,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            11 => Op::Data {
                op: DataOp::Cmn,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            12 => Op::Data {
                op: DataOp::Orr,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            13 => Op::Mul {
                s: true,
                rd,
                rm: rs,
                rs: rd,
            },
            14 => Op::Data {
                op: DataOp::Bic,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            15 => Op::Data {
                op: DataOp::Mvn,
                s: true,
                rd,
                rn: rd,
                op2: Op2::register(rs),
            },
            16.. => unreachable!(),
        };

        Instr { cc: Cc::Always, op }
    }
    fn decode_ldm_stm(self) -> Instr {
        let l = self.get_flag(11);
        let rb = self.get_reg(8, false);
        let regs = self.0 & 0xFF;

        Instr {
            cc: Cc::Always,
            op: Op::BlockTransfer {
                p: false,
                u: true,
                s: false,
                w: true,
                l,
                rn: rb,
                regs,
            },
        }
    }
    fn decode_pc_rel_load(self) -> Instr {
        let offset = self.0 as u8;
        let rd = self.get_reg(8, false);

        Instr {
            cc: Cc::Always,
            op: Op::LdrPc { rd, offset },
        }
    }
    fn decode_ldr_str_imm(self) -> Instr {
        let b = self.get_flag(12);
        let l = self.get_flag(11);
        let offset = (self.0 >> 6) & 0x1F;
        let rb = self.get_reg(3, false);
        let rd = self.get_reg(0, false);

        let offset = if !b { offset * 4 } else { offset };

        Instr {
            cc: Cc::Always,
            op: Op::WordTransfer {
                p: true,
                u: true,
                b,
                w: false,
                l,
                rn: rb,
                rd,
                offset: WordOffset::Immediate(offset),
            },
        }
    }
    fn decode_ldr_str_reg(self) -> Instr {
        let l = self.get_flag(11);
        let b = self.get_flag(10);
        let ro = self.get_reg(6, false);
        let rb = self.get_reg(3, false);
        let rd = self.get_reg(0, false);

        Instr {
            cc: Cc::Always,
            op: Op::WordTransfer {
                p: true,
                u: true,
                b,
                w: false,
                l,
                rn: rb,
                rd,
                offset: WordOffset::Shift(Shift {
                    rm: ro,
                    shift_type: ShiftType::LogicalLeft,
                    shift_count: ShiftCount::Constant(0),
                }),
            },
        }
    }
    fn decode_sp_rel_ldr_str(self) -> Instr {
        let l = self.get_flag(11);
        let rd = self.get_reg(8, false);
        let offset = (self.0 & 0xFF) * 4;

        Instr {
            cc: Cc::Always,
            op: Op::WordTransfer {
                p: true,
                u: true,
                b: false,
                w: false,
                l,
                rn: Reg::STACK,
                rd,
                offset: WordOffset::Immediate(offset),
            },
        }
    }
    fn decode_load_store_half(self) -> Instr {
        let l = self.get_flag(11);
        let offset = (self.0 >> 6) as u8 & 0x1F;
        let offset = offset * 2;
        let rb = self.get_reg(3, false);
        let rd = self.get_reg(0, false);

        Instr {
            cc: Cc::Always,
            op: Op::HalfwordTransfer {
                p: true,
                u: true,
                w: false,
                l,
                s: false,
                h: true,
                rn: rb,
                rd,
                offset: HalfOffset::Immediate(offset),
            },
        }
    }
    fn decode_load_store_signed_half(self) -> Instr {
        let h = self.get_flag(11);
        let s = self.get_flag(10);
        let ro = self.get_reg(6, false);
        let rb = self.get_reg(3, false);
        let rd = self.get_reg(0, false);

        let (l, s, h) = match (s, h) {
            (false, false) => (false, false, true),
            (_, _) => (true, s, h),
        };

        Instr {
            cc: Cc::Always,
            op: Op::HalfwordTransfer {
                p: true,
                u: true,
                w: false,
                l,
                s,
                h,
                rn: rb,
                rd,
                offset: HalfOffset::Register(ro),
            },
        }
    }
    fn decode_shift(self) -> Instr {
        let op = (self.0 >> 11) & 3;
        let shift_type = match op {
            0 => ShiftType::LogicalLeft,
            1 => ShiftType::LogicalRight,
            2 => ShiftType::ArithmeticRight,
            _ => unreachable!(),
        };
        let shift_amount = (self.0 >> 6) as u8 & 0x1F;
        let rs = self.get_reg(3, false);
        let rd = self.get_reg(0, false);

        Instr {
            cc: Cc::Always,
            op: Op::Data {
                op: DataOp::Mov,
                s: true,
                rd,
                rn: rs,
                op2: Op2::Register(Shift {
                    rm: rs,
                    shift_type,
                    shift_count: ShiftCount::Constant(shift_amount),
                }),
            },
        }
    }
    fn decode_alu_imm(self) -> Instr {
        let op = (self.0 >> 11) & 3;
        let rd = self.get_reg(8, false);
        let imm = self.0 as u8;

        let op = match op {
            0 => DataOp::Mov,
            1 => DataOp::Cmp,
            2 => DataOp::Add,
            3 => DataOp::Sub,
            4.. => unreachable!(),
        };

        Instr {
            cc: Cc::Always,
            op: Op::Data {
                op,
                s: true,
                rd,
                rn: rd,
                op2: Op2::Immediate(imm, 0),
            },
        }
    }
    fn decode_push_pop(self) -> Instr {
        let l = self.get_flag(11);
        let r = self.get_flag(8);
        let regs = self.0 & 0xFF;

        let mask = if l && r {
            0x8000
        } else if r {
            0x4000
        } else {
            0
        };
        let regs = regs | mask;

        let (p, u) = if l { (false, true) } else { (true, false) };

        Instr {
            cc: Cc::Always,
            op: Op::BlockTransfer {
                p,
                u,
                s: false,
                w: true,
                l,
                rn: Reg::STACK,
                regs,
            },
        }
    }
}

fn get_cc(nibble: u8) -> Cc {
    match nibble {
        0 => Cc::Equal,
        1 => Cc::NotEqual,
        2 => Cc::UnsignedHigherOrSame,
        3 => Cc::UnsignedLower,
        4 => Cc::Negative,
        5 => Cc::PositiveOrZero,
        6 => Cc::Overflow,
        7 => Cc::NoOverflow,
        8 => Cc::UnsignedHigher,
        9 => Cc::UnsignedLowerOrSame,
        10 => Cc::GreaterOrEqual,
        11 => Cc::LessThan,
        12 => Cc::GreaterThan,
        13 => Cc::LessThanOrEqual,
        14 => Cc::Always,
        15 => Cc::Reserved,
        _ => unreachable!(),
    }
}
