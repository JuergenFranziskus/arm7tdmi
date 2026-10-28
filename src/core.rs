use std::fmt::Display;

use crate::instruction::{PsrFields, Reg};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Core {
    pub regs: [u32; 16],
    pub regs_fiq: [u32; 7],
    pub regs_svc: [u32; 2],
    pub regs_abt: [u32; 2],
    pub regs_irq: [u32; 2],
    pub regs_und: [u32; 2],

    pub cpsr: Cpsr,
    pub spsr_fiq: Cpsr,
    pub spsr_svc: Cpsr,
    pub spsr_abt: Cpsr,
    pub spsr_irq: Cpsr,
    pub spsr_und: Cpsr,
}
impl Core {
    pub fn new() -> Self {
        Self {
            regs: [0; _],
            regs_fiq: [0; _],
            regs_svc: [0; _],
            regs_abt: [0; _],
            regs_irq: [0; _],
            regs_und: [0; _],

            cpsr: Cpsr::new(),
            spsr_fiq: Cpsr::new(),
            spsr_svc: Cpsr::new(),
            spsr_abt: Cpsr::new(),
            spsr_irq: Cpsr::new(),
            spsr_und: Cpsr::new(),
        }
    }

    pub fn get_reg(&self, reg: Reg) -> u32 {
        let code = reg.0 as usize;
        match self.cpsr.mode() {
            _ if code == 15 => self.regs[15],
            Mode::Fiq if code > 7 => self.regs_fiq[code - 8],
            Mode::Irq if code > 12 => self.regs_irq[code - 13],
            Mode::Svc if code > 12 => self.regs_svc[code - 13],
            Mode::Abt if code > 12 => self.regs_abt[code - 13],
            Mode::Und if code > 12 => self.regs_und[code - 13],
            _ => self.regs[code],
        }
    }
    pub fn get_reg_usr(&self, reg: Reg) -> u32 {
        self.regs[reg.0 as usize]
    }
    pub fn set_reg(&mut self, reg: Reg, to: u32) {
        let code = reg.0 as usize;
        match self.cpsr.mode() {
            _ if code == 15 => self.regs[15] = to,
            Mode::Fiq if code > 7 => self.regs_fiq[code - 8] = to,
            Mode::Irq if code > 12 => self.regs_irq[code - 13] = to,
            Mode::Svc if code > 12 => self.regs_svc[code - 13] = to,
            Mode::Abt if code > 12 => self.regs_abt[code - 13] = to,
            Mode::Und if code > 12 => self.regs_und[code - 13] = to,
            _ => self.regs[code] = to,
        }
    }
    pub fn set_reg_usr(&mut self, reg: Reg, to: u32) {
        self.regs[reg.0 as usize] = to;
    }

    pub fn curr_spsr(&self) -> &Cpsr {
        match self.cpsr.mode() {
            Mode::Usr => todo!(),
            Mode::Fiq => &self.spsr_fiq,
            Mode::Irq => &self.spsr_irq,
            Mode::Svc => &self.spsr_svc,
            Mode::Abt => &self.spsr_abt,
            Mode::Und => &self.spsr_und,
            Mode::Sys => todo!(),
        }
    }
    pub fn curr_spsr_mut(&mut self) -> &mut Cpsr {
        match self.cpsr.mode() {
            Mode::Usr => todo!(),
            Mode::Fiq => &mut self.spsr_fiq,
            Mode::Irq => &mut self.spsr_irq,
            Mode::Svc => &mut self.spsr_svc,
            Mode::Abt => &mut self.spsr_abt,
            Mode::Und => &mut self.spsr_und,
            Mode::Sys => todo!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cpsr(pub(crate) u32);
impl Cpsr {
    fn new() -> Self {
        let mut out = Self(0);
        out.set_m0(true);
        out.set_m1(true);
        out.set_m2(false);
        out.set_m3(false);
        out.set_m4(true);

        out.set_i(true);
        out.set_f(true);

        out
    }

    pub fn do_msr(&mut self, src: u32, fields: PsrFields, usr: bool, is_cpsr: bool) {
        let usr_mask = if usr { 0xFF000000 } else { 0xFFFFFFFF };
        let field_mask = fields.mask();
        let mask = usr_mask & field_mask;
        let src = src & mask;
        self.0 &= !mask;
        self.0 |= src;

        if is_cpsr {
            self.set_m4(true);
        }
    }

    const M0: u32 = 1 << 0;
    const M1: u32 = 1 << 1;
    const M2: u32 = 1 << 2;
    const M3: u32 = 1 << 3;
    const M4: u32 = 1 << 4;
    const T: u32 = 1 << 5;
    const F: u32 = 1 << 6;
    const I: u32 = 1 << 7;
    const V: u32 = 1 << 28;
    const C: u32 = 1 << 29;
    const Z: u32 = 1 << 30;
    const N: u32 = 1 << 31;

    pub fn mode(self) -> Mode {
        match self.m() {
            0b10000 => Mode::Usr,
            0b10001 => Mode::Fiq,
            0b10010 => Mode::Irq,
            0b10011 => Mode::Svc,
            0b10111 => Mode::Abt,
            0b11011 => Mode::Und,
            0b11111 => Mode::Sys,
            or => panic!("{or:b} is not a valid system mode"),
        }
    }
    pub fn m(self) -> u8 {
        (self.0 & 0x1F) as u8
    }
    pub fn set_m(&mut self, to: u8) {
        let to = to & 0x1F;
        self.0 &= !0x1F;
        self.0 |= to as u32;
    }

    pub fn m0(self) -> bool {
        self.0 & Self::M0 != 0
    }
    pub fn m1(self) -> bool {
        self.0 & Self::M1 != 0
    }
    pub fn m2(self) -> bool {
        self.0 & Self::M2 != 0
    }
    pub fn m3(self) -> bool {
        self.0 & Self::M3 != 0
    }
    pub fn m4(self) -> bool {
        self.0 & Self::M4 != 0
    }
    pub fn t(self) -> bool {
        self.0 & Self::T != 0
    }
    pub fn f(self) -> bool {
        self.0 & Self::F != 0
    }
    pub fn i(self) -> bool {
        self.0 & Self::I != 0
    }
    pub fn v(self) -> bool {
        self.0 & Self::V != 0
    }
    pub fn c(self) -> bool {
        self.0 & Self::C != 0
    }
    pub fn z(self) -> bool {
        self.0 & Self::Z != 0
    }
    pub fn n(self) -> bool {
        self.0 & Self::N != 0
    }

    pub fn set_m0(&mut self, to: bool) {
        self.0 &= !Self::M0;
        if to {
            self.0 |= Self::M0;
        }
    }
    pub fn set_m1(&mut self, to: bool) {
        self.0 &= !Self::M1;
        if to {
            self.0 |= Self::M1;
        }
    }
    pub fn set_m2(&mut self, to: bool) {
        self.0 &= !Self::M2;
        if to {
            self.0 |= Self::M2;
        }
    }
    pub fn set_m3(&mut self, to: bool) {
        self.0 &= !Self::M3;
        if to {
            self.0 |= Self::M3;
        }
    }
    pub fn set_m4(&mut self, to: bool) {
        self.0 &= !Self::M4;
        if to {
            self.0 |= Self::M4;
        }
    }
    pub fn set_t(&mut self, to: bool) {
        self.0 &= !Self::T;
        if to {
            self.0 |= Self::T;
        }
    }
    pub fn set_f(&mut self, to: bool) {
        self.0 &= !Self::F;
        if to {
            self.0 |= Self::F;
        }
    }
    pub fn set_i(&mut self, to: bool) {
        self.0 &= !Self::I;
        if to {
            self.0 |= Self::I;
        }
    }
    pub fn set_v(&mut self, to: bool) {
        self.0 &= !Self::V;
        if to {
            self.0 |= Self::V;
        }
    }
    pub fn set_c(&mut self, to: bool) {
        self.0 &= !Self::C;
        if to {
            self.0 |= Self::C;
        }
    }
    pub fn set_z(&mut self, to: bool) {
        self.0 &= !Self::Z;
        if to {
            self.0 |= Self::Z;
        }
    }
    pub fn set_n(&mut self, to: bool) {
        self.0 &= !Self::N;
        if to {
            self.0 |= Self::N;
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        match mode {
            Mode::Usr => self.set_m(0b10000),
            Mode::Fiq => self.set_m(0b10001),
            Mode::Irq => self.set_m(0b10010),
            Mode::Svc => self.set_m(0b10011),
            Mode::Abt => self.set_m(0b10111),
            Mode::Und => self.set_m(0b11011),
            Mode::Sys => self.set_m(0b11111),
        }
    }
}
impl Display for Cpsr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt = f;
        let n = if self.n() { "N" } else { " " };
        let z = if self.z() { "Z" } else { " " };
        let c = if self.c() { "C" } else { " " };
        let v = if self.v() { "V" } else { " " };
        let i = if self.i() { "I" } else { " " };
        let f = if self.f() { "F" } else { " " };
        let t = if self.t() { "T" } else { " " };

        write!(fmt, "{n}{z}{c}{v}-{i}{f}{t}{:0>5b}", self.m())?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Usr,
    Fiq,
    Irq,
    Svc,
    Abt,
    Und,
    Sys,
}
impl Mode {
    pub fn has_spsr(&self) -> bool {
        match self {
            Mode::Fiq => true,
            Mode::Irq => true,
            Mode::Svc => true,
            Mode::Abt => true,
            Mode::Und => true,
            Mode::Usr => false,
            Mode::Sys => false,
        }
    }
}
