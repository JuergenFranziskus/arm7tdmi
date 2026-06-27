use serde::Deserialize;

use crate::{AccessSize, Arm7TDMI, Bus, CycleType, core::{Core, Cpsr}, instruction::{Instruction, Operation, Register}};

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

#[derive(Deserialize)]
#[derive(Copy, Clone, Debug)]
pub struct Transaction {
    pub kind: u8,
    pub size: u8,
    pub addr: u32,
    pub data: u32,
    pub cycle: u8,
    pub access: u8,
}


pub struct TestBus<'a> {
    test: &'a Test,
    trans: usize,
}
impl<'a> TestBus<'a> {
    pub fn new(test: &'a Test) -> Self {
        Self {
            test,
            trans: 0,
        }
    }

    fn do_read(&self, addr: u32, size: AccessSize) -> u32 {
        let trans = &self.test.transactions[self.trans];
        if trans.addr != addr { panic!("Is {addr:0>8x}, should {:0>8x}", trans.addr); }
        trans.data
    }
}
impl<'a> Bus for TestBus<'a> {
    fn cycle(&mut self, addr: u32, data: u32, size: AccessSize, ty: CycleType, rw: bool, cpu: &Arm7TDMI) -> u32 {
        if ty == CycleType::Internal { return addr; }

        let seq = ty == CycleType::Sequential;

        let data = if rw {
            self.do_read(addr, size)
        }
        else {
            data  
        };
        print_debug(cpu, addr, Some(data), Some(rw), Some(size), ty);
        verify_transaction(addr, data, rw, size, seq, &self.test, self.trans);
        self.trans += 1;

        data
    }
}



pub fn run_test(test: &Test) {
    let mut cpu = init_cpu(test);
    let mut bus = TestBus::new(test);

    let (_, instr) = cpu.curr_instr();

    print_start_debug(&cpu);
    cpu.run(&mut bus);

    {
        verify_end(&cpu, &test.end);
        if bus.trans < test.transactions.len() {
            eprintln!("Did not consume all {} transactions, only {}", test.transactions.len(), bus.trans);
            panic!();
        }
    }
}
fn init_cpu(test: &Test) -> Arm7TDMI {
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

    Arm7TDMI::init_test(core, start.pipeline, test.base_addr)
}
fn verify_transaction(addr: u32, data: u32, rw: bool, size: AccessSize, seq: bool, test: &Test, i: usize) -> bool {
    let t = &test.transactions[i];
    let mut err = false;


    let is_size = match size {
        AccessSize::Byte => 1,
        AccessSize::Half => 2,
        AccessSize::Word => 4,
    };
    let should_size = t.size;
    if is_size != should_size {
        err = true;
        eprintln!("Access size does not match for cycle {}, should {should_size}, is {is_size}", t.cycle);
    }

    let is_addr = addr;
    let should_addr = t.addr;
    if is_addr != should_addr {
        err = true;
        eprintln!("Address bus does not match for cycle {}, should {should_addr:0>8x}, is {is_addr:0>8x}", t.cycle);
    }

    let is_data = data;
    let should_data = t.data;
    if is_data != should_data {
        err = true;
        eprintln!("Data bus does not match for cycle {}, should {should_data:0>8x}, is {is_data:0>8x}", t.cycle);
    }

    // let is_kind = if rw { 1 } else { 2 };
    // let should_kind = t.kind & 0x3;
    // if is_kind != should_kind {
        // err = true;
        // eprintln!("Access kind does not match for cycle {}, should {should_kind}, is {is_kind}", t.cycle);
    // }

    // let is_sequential = if bus.cycle_type == CycleType::Sequential { true } else { false };
    // let should_sequential = t.access & 1 != 0;
    // if is_sequential != should_sequential {
        // err = true;
        // eprintln!("Sequentiality does not match for cycle {}, should {should_sequential}, is {is_sequential}", t.cycle);
    // }


    if err { panic!() };

    true
}
fn verify_end(cpu: &Arm7TDMI, end: &State) {
    let mut err = false;
    let core = cpu.core();
    for (i, (is, should)) in core.regs.iter().copied().zip(end.regs.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }

    for (i, (is, should)) in core.regs_irq.iter().copied().zip(end.regs_irq.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("IRQ R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }
    for (i, (is, should)) in core.regs_abt.iter().copied().zip(end.regs_abt.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("ABT R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }
    for (i, (is, should)) in core.regs_svc.iter().copied().zip(end.regs_svc.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("SVC R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }
    for (i, (is, should)) in core.regs_und.iter().copied().zip(end.regs_und.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("UND R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }
    for (i, (is, should)) in core.regs_fiq.iter().copied().zip(end.regs_fiq.iter().copied()).enumerate() {
        if is != should {
            err = true;
            eprintln!("FIQ R{i} does not match, is {is:0>8x}, should {should:0>8x}");
        }
    }

    let is_cpsr = core.cpsr;
    let should_cpsr = Cpsr(end.cpsr);
    if is_cpsr != should_cpsr {
        err = true;
        eprintln!("CPSR does not match, is {is_cpsr} / {:0>8x}, should {should_cpsr} / {:0>8x}", is_cpsr.0, should_cpsr.0);
    }


    for i in 0..2 {
        let is_pipe = cpu.pipeline[i];
        let should_pipe = end.pipeline[i];
        if is_pipe != should_pipe {
            err = true;
            eprintln!("Pipeline[{i}] does not match, is {is_pipe:0>8x}, should {should_pipe:0>8x}");
        }
    }



    if err { panic!(); }
}


const DEBUG: bool = true;
fn print_start_debug(cpu: &Arm7TDMI) {
    if !DEBUG { return; }

    print!("    ");
    for i in 0..16 {
        print!("R{i:0>2}     = {:0>8x}", cpu.core.regs[i]);
        if i != 7 && i != 15 {
            print!(", ");
        }
        if i == 7 {
            print!("\n    ");
        }
    }
    print!("\n    ");
    for i in 0..7 {
        print!("R{:0>2}_fiq = {:0>8x}", i + 8, cpu.core.regs_fiq[i]);
        if i != 6 {
            print!(", ");
        }
    }
    print!("\n    ");

    for i in 0..2 {
        print!("R{:0>2}_irq = {:0>8x}", i + 13, cpu.core.regs_irq[i]);
        if i != 1 {
            print!(", ");
        }
    }
    print!("\n    ");

    for i in 0..2 {
        print!("R{:0>2}_svc = {:0>8x}", i + 13, cpu.core.regs_svc[i]);
        if i != 1 {
            print!(", ");
        }
    }
    print!("\n    ");

    for i in 0..2 {
        print!("R{:0>2}_abt = {:0>8x}", i + 13, cpu.core.regs_abt[i]);
        if i != 1 {
            print!(", ");
        }
    }
    print!("\n    ");

    for i in 0..2 {
        print!("R{:0>2}_und = {:0>8x}", i + 13, cpu.core.regs_und[i]);
        if i != 1 {
            print!(", ");
        }
    }
    println!();



    let (addr, instr) = cpu.curr_instr();

    let disasm = super::instruction::print::format(instr, Some(cpu.core()), Some(addr));
    println!("{addr:0>8x} {disasm}");
}
fn print_debug(cpu: &Arm7TDMI, addr: u32, data: Option<u32>, rw: Option<bool>, size: Option<AccessSize>, cycle_ty: CycleType) {
    if !DEBUG { return; }

    print!("    ");

    let ty = match cycle_ty {
        CycleType::Internal => "I",
        CycleType::NonSequential => "N",
        CycleType::Sequential => "S",
    };

    let rw = match rw {
        Some(false) => "w",
        Some(true) => "R",
        None => " ",
    };
    let addr = addr;
    let data = data.map(|d| format!("{d:0>8x}")).unwrap_or(String::from("        "));
    let size = match size {
        Some(AccessSize::Byte) => "B",
        Some(AccessSize::Half) => "H",
        Some(AccessSize::Word) => "W",
        None => " ",
    };
    print!("{ty}{size} {addr:0>8x} {rw} {data}        ");

    let core = cpu.core();
    let cpsr = core.cpsr;

    let spsr = core.spsr();

    print!("{cpsr} {:?} / {spsr} {:?}", cpsr.mode(), spsr.mode());


    println!();
}
