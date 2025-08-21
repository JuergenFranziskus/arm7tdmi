

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Core {
    pub registers: [u32; 16],
    pub registers_fiq: [u32; 7],
    pub registers_svc: [u32; 2],
    pub registers_abt: [u32; 2],
    pub registers_irq: [u32; 2],
    pub registers_und: [u32; 2],

    pub cpsr: Cpsr,
    pub spsr_fiq: Cpsr,
    pub spsr_svc: Cpsr,
    pub spsr_abt: Cpsr,
    pub spsr_irq: Cpsr,
    pub spsr_und: Cpsr,
}
impl Core {
    
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cpsr(u32);
impl Cpsr {
    const M0: u32 = 1 << 0;
    const M1: u32 = 1 << 1;
    const M2: u32 = 1 << 2;
    const M3: u32 = 1 << 3;
    const T: u32 = 1 << 4;
    const F: u32 = 1 << 5;
    const I: u32 = 1 << 6;
    const V: u32 = 1 << 28;
    const C: u32 = 1 << 29;
    const Z: u32 = 1 << 30;
    const N: u32 = 1 << 31;

    pub fn m0(self) -> bool { self.0 & Self::M0 != 0}
    pub fn m1(self) -> bool { self.0 & Self::M1 != 0}
    pub fn m2(self) -> bool { self.0 & Self::M2 != 0}
    pub fn m3(self) -> bool { self.0 & Self::M3 != 0}
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
    pub fn with_t(mut self, to: bool) -> Self { self.set_t(to); self }
    pub fn with_f(mut self, to: bool) -> Self { self.set_f(to); self }
    pub fn with_i(mut self, to: bool) -> Self { self.set_i(to); self }
    pub fn with_v(mut self, to: bool) -> Self { self.set_v(to); self }
    pub fn with_c(mut self, to: bool) -> Self { self.set_c(to); self }
    pub fn with_z(mut self, to: bool) -> Self { self.set_z(to); self }
    pub fn with_n(mut self, to: bool) -> Self { self.set_n(to); self }
}
