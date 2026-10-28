use serde::Deserialize;

use crate::{
    Arm7, Bus, Size,
    core::{Core, Cpsr},
    instruction::{Instr, print::format},
};

#[derive(Deserialize)]
pub struct Test {
    #[serde(rename = "initial")]
    pub start: State,
    #[serde(rename = "final")]
    pub end: State,
    pub transactions: Vec<Transaction>,
    pub opcode: u32,
    pub base_addr: u32,
}

#[derive(Deserialize)]
pub struct State {
    #[serde(rename = "R")]
    pub regs: [u32; 16],
    #[serde(rename = "R_fiq")]
    pub regs_fiq: [u32; 7],
    #[serde(rename = "R_svc")]
    pub regs_svc: [u32; 2],
    #[serde(rename = "R_abt")]
    pub regs_abt: [u32; 2],
    #[serde(rename = "R_irq")]
    pub regs_irq: [u32; 2],
    #[serde(rename = "R_und")]
    pub regs_und: [u32; 2],
    #[serde(rename = "CPSR")]
    pub cpsr: u32,
    #[serde(rename = "SPSR")]
    pub spsr: [u32; 5],
    pub pipeline: [u32; 2],
}

#[derive(Deserialize, Copy, Clone, Debug)]
pub struct Transaction {
    pub kind: u8,
    pub size: u8,
    pub addr: u32,
    pub data: u32,
    pub cycle: u8,
    pub access: u8,
}

pub fn do_test(test: &Test) {
    let mut cpu = init_cpu(test);
    let mut bus = TestBus {
        test,
        transaction: 0,

        instr: None,
        ignore_c: false,
        ignore_v: false,
    };

    cpu.exec(&mut bus);

    check_end(&cpu, &bus, test, bus.transaction);
}
fn init_cpu(test: &Test) -> Arm7 {
    let start = &test.start;

    let core = Core {
        regs: start.regs,
        regs_fiq: start.regs_fiq,
        regs_svc: start.regs_svc,
        regs_abt: start.regs_abt,
        regs_irq: start.regs_irq,
        regs_und: start.regs_und,
        cpsr: Cpsr(start.cpsr),
        spsr_fiq: Cpsr(start.spsr[0]),
        spsr_svc: Cpsr(start.spsr[1]),
        spsr_abt: Cpsr(start.spsr[2]),
        spsr_irq: Cpsr(start.spsr[3]),
        spsr_und: Cpsr(start.spsr[4]),
    };

    Arm7::new(core, start.pipeline)
}
fn check_end(cpu: &Arm7, bus: &TestBus, test: &Test, transaction: usize) {
    let end = &test.end;

    let core = &cpu.core;
    let mut err = false;

    for i in 0..16 {
        if core.regs[i] != end.regs[i] {
            eprintln!(
                "R{i} does not match, should {:0>8x}, is {:0>8x}",
                end.regs[i], core.regs[i]
            );
            err = true;
        }
    }
    for i in 0..7 {
        if core.regs_fiq[i] != end.regs_fiq[i] {
            eprintln!(
                "R{}_fiq does not match, should {:0>8x}, is {:0>8x}",
                i + 8,
                end.regs_fiq[i],
                core.regs_fiq[i]
            );
            err = true;
        }
    }
    for i in 0..2 {
        if core.regs_svc[i] != end.regs_svc[i] {
            eprintln!(
                "R{}_svc does not match, should {:0>8x}, is {:0>8x}",
                i + 13,
                end.regs_svc[i],
                core.regs_svc[i]
            );
            err = true;
        }
    }
    for i in 0..2 {
        if core.regs_abt[i] != end.regs_abt[i] {
            eprintln!(
                "R{}_abt does not match, should {:0>8x}, is {:0>8x}",
                i + 13,
                end.regs_abt[i],
                core.regs_abt[i]
            );
            err = true;
        }
    }
    for i in 0..2 {
        if core.regs_irq[i] != end.regs_irq[i] {
            eprintln!(
                "R{}_irq does not match, should {:0>8x}, is {:0>8x}",
                i + 13,
                end.regs_irq[i],
                core.regs_irq[i]
            );
            err = true;
        }
    }
    for i in 0..2 {
        if core.regs_und[i] != end.regs_und[i] {
            eprintln!(
                "R{}_und does not match, should {:0>8x}, is {:0>8x}",
                i + 13,
                end.regs_und[i],
                core.regs_und[i]
            );
            err = true;
        }
    }

    let psrs = [
        (core.cpsr, Cpsr(end.cpsr), "CPSR"),
        (core.spsr_fiq, Cpsr(end.spsr[0]), "SPSR_fiq"),
        (core.spsr_svc, Cpsr(end.spsr[1]), "SPSR_svc"),
        (core.spsr_abt, Cpsr(end.spsr[2]), "SPSR_abt"),
        (core.spsr_irq, Cpsr(end.spsr[3]), "SPSR_irq"),
        (core.spsr_und, Cpsr(end.spsr[4]), "SPSR_und"),
    ];

    let c_mask = if bus.ignore_c { !(1 << 29) } else { !0 };
    let v_mask = if bus.ignore_v { !(1 << 28) } else { !0 };
    let mask = c_mask & v_mask;

    for (is, should, name) in psrs {
        if is.0 & mask != should.0 & mask {
            eprintln!("{name} does not match, should {should}, is {is}");
            err = true;
        }
    }

    if transaction < test.transactions.len() {
        eprintln!(
            "Not all transactions have been consumed, has {}, used {}",
            test.transactions.len(),
            transaction
        );
        err = true;
    }

    for i in 0..2 {
        if cpu.pipe[i].0 != end.pipeline[i] {
            eprintln!(
                "Pipeline[{i}] does not match, should {:0>8x}, is {:0>8x}",
                end.pipeline[i], cpu.pipe[i].0
            );
            err = true;
        }
    }

    if err {
        panic!("Final state does not match");
    }
}

struct TestBus<'a> {
    test: &'a Test,
    transaction: usize,

    instr: Option<Instr>,
    ignore_c: bool,
    ignore_v: bool,
}
impl<'a> TestBus<'a> {
    fn handle_access(
        &mut self,
        addr: u32,
        data: u32,
        rw: bool,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) -> u32 {
        if rw && opc {
            self.handle_opcode_access(addr, data, size, seq)
        } else {
            self.handle_transaction(addr, data, rw, size, opc, seq, lock)
        }
    }
    fn handle_opcode_access(&mut self, addr: u32, data: u32, size: Size, seq: bool) -> u32 {
        if self.test.transactions[self.transaction].addr == addr {
            self.handle_transaction(addr, data, true, size, true, seq, false)
        } else if addr == self.test.base_addr {
            self.test.opcode
        } else {
            addr
        }
    }
    fn handle_transaction(
        &mut self,
        addr: u32,
        data: u32,
        rw: bool,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) -> u32 {
        let trans = &self.test.transactions[self.transaction];
        self.transaction += 1;

        let mut err = false;
        let data = if trans.kind == 2 { data } else { trans.data };

        let should_rw = trans.kind != 2;
        let is_size = match size {
            Size::Byte => 1,
            Size::Half => 2,
            Size::Word => 4,
        };
        let should_opc = trans.kind == 0;
        //let should_seq = trans.access & 1 != 0;
        let _ = seq;
        let should_lock = trans.access & 8 != 0;

        if addr != trans.addr {
            eprintln!(
                "ADDR does not match, should {:0>8x}, is {addr:0>8x}",
                trans.addr
            );
            err = true;
        }
        if data != trans.data {
            eprintln!(
                "DATA does not match, should {:0>8x}, is {data:0>8x}",
                trans.data
            );
            err = true;
        }
        if rw != should_rw {
            eprintln!("RW   does not match, should {should_rw}, is {rw}");
            err = true;
        }
        if is_size != trans.size {
            eprintln!("SIZE does not match, should {}, is {is_size}", trans.size);
            err = true;
        }
        if opc != should_opc {
            eprintln!("OPC  does not match, should {should_opc}, is {opc}");
            err = true;
        }
        //if seq != should_seq {
        //    eprintln!("SEQ  does not match, should {should_seq}, is {seq}");
        //    err = true;
        //}
        if lock != should_lock {
            eprintln!("LOCK does not match, should {should_lock}, is {lock}");
            err = true;
        }

        if err {
            panic!("Bus did not match");
        }

        data
    }
}
impl<'a> Bus for TestBus<'a> {
    fn access(
        &mut self,
        addr: u32,
        data: u32,
        rw: bool,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) -> u32 {
        let data = if size == Size::Byte && !rw {
            data & 0xFF
        } else if size == Size::Half && !rw {
            data & 0xFFFF
        } else {
            data
        };

        let data = self.handle_access(addr, data, rw, size, opc, seq, lock);
        let lo = addr & 3;
        let rot = lo * 8;
        if size == Size::Byte && rw {
            data.rotate_left(rot)
        } else if size == Size::Half && rw {
            data.rotate_left(rot)
        } else {
            data
        }
    }

    fn io(&mut self, addr: u32) {
        let _ = addr;
    }

    fn dbg_start_instr(&mut self, instr: Instr, addr: u32, core: &Core) {
        let _ = addr;
        let _ = core;
        self.instr = Some(instr);
    }

    fn dbg_load(
        &mut self,
        core: &Core,
        addr: u32,
        data: u32,
        size: Size,
        opc: bool,
        seq: bool,
        lock: bool,
    ) {
        let _ = core;
        print_dbg(
            self.instr.take(),
            addr,
            data,
            size,
            true,
            seq,
            lock,
            opc,
            true,
        );
    }

    fn dbg_store(&mut self, core: &Core, addr: u32, data: u32, size: Size, seq: bool, lock: bool) {
        let _ = core;
        print_dbg(
            self.instr.take(),
            addr,
            data,
            size,
            true,
            seq,
            lock,
            false,
            false,
        );
    }

    fn dbg_io(&mut self, core: &Core, addr: u32) {
        let _ = core;
        print_dbg(
            self.instr.take(),
            addr,
            0,
            Size::Word,
            false,
            false,
            false,
            false,
            true,
        );
    }

    fn dbg_c_unpredictable(&mut self) {
        self.ignore_c = true;
    }

    fn dbg_v_unpredictable(&mut self) {
        self.ignore_v = true;
    }
}

fn print_dbg(
    instr: Option<Instr>,
    addr: u32,
    data: u32,
    size: Size,
    mreq: bool,
    seq: bool,
    lock: bool,
    opc: bool,
    rw: bool,
) {
    let instr = if let Some(instr) = instr {
        format(instr, None, None)
    } else {
        String::new()
    };

    let data = if mreq {
        format!("{data:0>8x}")
    } else {
        String::new()
    };
    let size = match size {
        Size::Byte => "BYTE",
        Size::Half => "HALF",
        Size::Word => "WORD",
    };
    let size = if mreq { size } else { "    " };
    let seq = if mreq && seq { "SEQ" } else { "   " };
    let lock = if mreq && lock { "LOCK" } else { "    " };
    let opc = if mreq && rw && opc { "OPC" } else { "   " };
    let kind = if mreq && rw {
        "LD"
    } else if mreq {
        "ST"
    } else {
        "IO"
    };

    println!("{instr:<30}{opc} {addr:0>8x} {seq} {kind} {size} {data} {lock}");
}
