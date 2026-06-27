use std::{fmt::Display, ops::{Index, IndexMut}};

use crate::instruction::{CpsrFields, Register};



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


    pub fn spsr(&self) -> Cpsr {
        match self.cpsr.mode() {
            Mode::Fiq => self.spsr_fiq,
            Mode::Supervisor => self.spsr_svc,
            Mode::Abort => self.spsr_abt,
            Mode::Irq => self.spsr_irq,
            Mode::Undefined => self.spsr_und,
            Mode::User | Mode::System => self.cpsr,
        }
    }
    pub fn set_spsr(&mut self, to: Cpsr) {
        match self.cpsr.mode() {
            Mode::Fiq => self.spsr_fiq = to,
            Mode::Supervisor => self.spsr_svc = to,
            Mode::Abort => self.spsr_abt = to,
            Mode::Irq => self.spsr_irq = to,
            Mode::Undefined => self.spsr_und = to,
            Mode::User | Mode::System => (),
        }
    }
}
impl Index<Register> for Core {
    type Output = u32;

    fn index(&self, index: Register) -> &Self::Output {
        let i = index.0 as usize;
        match self.cpsr.mode() {
            Mode::Fiq if i >= 8&& i != 15  => &self.regs_fiq[i - 8],
            Mode::Irq if i >= 13&& i != 15 => &self.regs_irq[i - 13],
            Mode::Supervisor if i >= 13&& i != 15 => &self.regs_svc[i - 13],
            Mode::Abort if i >= 13&& i != 15 => &self.regs_abt[i - 13],
            Mode::Undefined if i >= 13&& i != 15 => &self.regs_und[i - 13],
            _ => &self.regs[i],
        }
    }
}
impl IndexMut<Register> for Core {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        let i = index.0 as usize;
        match self.cpsr.mode() {
            Mode::Fiq if i >= 8 && i != 15 => &mut self.regs_fiq[i - 8],
            Mode::Irq if i >= 13 && i != 15 => &mut self.regs_irq[i - 13],
            Mode::Supervisor if i >= 13 && i != 15 => &mut self.regs_svc[i - 13],
            Mode::Abort if i >= 13 && i != 15 => &mut self.regs_abt[i - 13],
            Mode::Undefined if i >= 13 && i != 15 => &mut self.regs_und[i - 13],
            _ => &mut self.regs[i],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cpsr(pub u32);
impl Cpsr {
    pub fn new() -> Self {
        Self(0)
            .with_m0(true)
            .with_m1(true)
            .with_m4(true)
            .with_i(true)
            .with_f(true)
    }


    pub fn msr(&mut self, data: u32, fields: CpsrFields, user: bool) {
        let old_t = self.t();
        let mask = fields.mask(user);
        let yes = data & mask;
        let no = self.0 & !mask;
        self.0 = yes | no;
        //self.set_t(old_t);
    }


    pub fn mode(self) -> Mode {
        match self.m() {
            0b10000 => Mode::User,
            0b10001 => Mode::Fiq,
            0b10010 => Mode::Irq,
            0b10011 => Mode::Supervisor,
            0b10111 => Mode::Abort,
            0b11011 => Mode::Undefined,
            0b11111 => Mode::System,
            err => Mode::User,
        }
    }
    pub fn is_mode_valid(self) -> bool {
        match self.m() {
            0b10000 => true,
            0b10001 => true,
            0b10010 => true,
            0b10011 => true,
            0b10111 => true,
            0b11011 => true,
            0b11111 => true,
            _ => false
        }
    }
    pub fn set_mode(&mut self, mode: Mode) {
        match mode {
            Mode::User => self.set_m(0b10000),
            Mode::Fiq => self.set_m(0b10001),
            Mode::Irq => self.set_m(0b10010),
            Mode::Supervisor => self.set_m(0b10011),
            Mode::Abort => self.set_m(0b10111),
            Mode::Undefined => self.set_m(0b11011),
            Mode::System => self.set_m(0b11111),
        }
    }

    pub fn l(self) -> u32 {
        if self.t() { 2 } else { 4 }
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

    pub fn m0(self) -> bool { self.0 & Self::M0 != 0}
    pub fn m1(self) -> bool { self.0 & Self::M1 != 0}
    pub fn m2(self) -> bool { self.0 & Self::M2 != 0}
    pub fn m3(self) -> bool { self.0 & Self::M3 != 0}
    pub fn m4(self) -> bool { self.0 & Self::M4 != 0}
    pub fn t(self) -> bool { self.0 & Self::T != 0}
    pub fn f(self) -> bool { self.0 & Self::F != 0}
    pub fn i(self) -> bool { self.0 & Self::I != 0}
    pub fn v(self) -> bool { self.0 & Self::V != 0}
    pub fn c(self) -> bool { self.0 & Self::C != 0}
    pub fn z(self) -> bool { self.0 & Self::Z != 0}
    pub fn n(self) -> bool { self.0 & Self::N != 0}

    pub fn set_m0(&mut self, to: bool) { self.0 &= !Self::M0; if to { self.0 |= Self::M0; }}
    pub fn set_m1(&mut self, to: bool) { self.0 &= !Self::M1; if to { self.0 |= Self::M1; }}
    pub fn set_m2(&mut self, to: bool) { self.0 &= !Self::M2; if to { self.0 |= Self::M2; }}
    pub fn set_m3(&mut self, to: bool) { self.0 &= !Self::M3; if to { self.0 |= Self::M3; }}
    pub fn set_m4(&mut self, to: bool) { self.0 &= !Self::M4; if to { self.0 |= Self::M4; }}
    pub fn set_t(&mut self, to: bool) { self.0 &= !Self::T; if to { self.0 |= Self::T; }}
    pub fn set_f(&mut self, to: bool) { self.0 &= !Self::F; if to { self.0 |= Self::F; }}
    pub fn set_i(&mut self, to: bool) { self.0 &= !Self::I; if to { self.0 |= Self::I; }}
    pub fn set_v(&mut self, to: bool) { self.0 &= !Self::V; if to { self.0 |= Self::V; }}
    pub fn set_c(&mut self, to: bool) { self.0 &= !Self::C; if to { self.0 |= Self::C; }}
    pub fn set_z(&mut self, to: bool) { self.0 &= !Self::Z; if to { self.0 |= Self::Z; }}
    pub fn set_n(&mut self, to: bool) { self.0 &= !Self::N; if to { self.0 |= Self::N; }}

    pub fn with_m0(mut self, to: bool) -> Self { self.set_m0(to); self }
    pub fn with_m1(mut self, to: bool) -> Self { self.set_m1(to); self }
    pub fn with_m2(mut self, to: bool) -> Self { self.set_m2(to); self }
    pub fn with_m3(mut self, to: bool) -> Self { self.set_m3(to); self }
    pub fn with_m4(mut self, to: bool) -> Self { self.set_m4(to); self }
    pub fn with_t(mut self, to: bool) -> Self { self.set_t(to); self }
    pub fn with_f(mut self, to: bool) -> Self { self.set_f(to); self }
    pub fn with_i(mut self, to: bool) -> Self { self.set_i(to); self }
    pub fn with_v(mut self, to: bool) -> Self { self.set_v(to); self }
    pub fn with_c(mut self, to: bool) -> Self { self.set_c(to); self }
    pub fn with_z(mut self, to: bool) -> Self { self.set_z(to); self }
    pub fn with_n(mut self, to: bool) -> Self { self.set_n(to); self }
    
    pub fn m(self) -> u8 {
        let m0 = (self.m0() as u8) << 0;
        let m1 = (self.m1() as u8) << 1;
        let m2 = (self.m2() as u8) << 2;
        let m3 = (self.m3() as u8) << 3;
        let m4 = (self.m4() as u8) << 4;
        let m = m0 | m1 | m2 | m3 | m4;
        m
    }
    pub fn set_m(&mut self, to: u8) {
        self.set_m0(to & 1 != 0);
        self.set_m1(to & 2 != 0);
        self.set_m2(to & 4 != 0);
        self.set_m3(to & 8 != 0);
        self.set_m4(to & 16 != 0);
        
    }
}
impl Display for Cpsr {
    fn fmt(&self, fr: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = self.m();
        let t = if self.t() { "T" } else { " " };
        let f = if self.f() { "F" } else { " " };
        let i = if self.i() { "I" } else { " " };
        let v = if self.v() { "V" } else { " " };
        let c = if self.c() { "C" } else { " " };
        let z = if self.z() { "Z" } else { " " };
        let n = if self.n() { "N" } else { " " };

        write!(fr, "{n}{z}{c}{v}-{i}{f}{t}-{m:0>5b}")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    User,
    Fiq,
    Irq,
    Supervisor,
    Abort,
    Undefined,
    System,
}
