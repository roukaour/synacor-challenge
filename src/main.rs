use std::env;
use std::io;
use std::io::Read;
use std::fs::File;

const MEM_SIZE: usize = 0x8000;
const NUM_REGISTERS: usize = 8;
const BASE: u16 = 0x8000;

macro_rules! fatal {
    ($($arg:expr),+) => {
        print!("\n*** FATAL ERROR: ");
        println!($($arg),+);
        panic!();
    };
}

struct VM {
    // memory with 15-bit address space storing 16-bit values
    memory: [u16; MEM_SIZE],
    // eight registers
    registers: [u16; NUM_REGISTERS],
    // an unbounded stack which holds individual 16-bit values
    stack: Vec<u16>,
    pc: usize,
}

impl VM {
    pub fn new() -> VM {
        VM {
            memory: [0; MEM_SIZE],
            registers: [0; NUM_REGISTERS],
            stack: vec!(),
            pc: 0,
        }
    }
    
    pub fn init(&mut self, program: &[u16]) {
        for (i, v) in program.iter().enumerate() {
            self.memory[i] = *v;
        }
    }
    
    pub fn load(&mut self, filename: String) -> io::Result<()> {
        let mut file = try!(File::open(filename));
        let mut buffer = [0; 2];
        // programs are loaded into memory starting at address 0
        let mut i = 0;
        loop {
            match file.read(&mut buffer) {
                // each number is stored as a 16-bit little-endian pair (low byte, high byte)
                Ok(2) => {
                    self.memory[i] = ((buffer[1] as u16) << 8) | buffer[0] as u16;
                    i += 1;
                }
                _ => { break; }
            };
        }
        Ok(())
    }
    
    fn get(&mut self) -> u16 {
        let a = self.memory[self.pc] as usize;
        self.pc += 1;
        if a < MEM_SIZE {
            // numbers 0..32767 mean a literal value
            a as u16
        } else if a - MEM_SIZE < NUM_REGISTERS {
            // numbers 32768..32775 instead mean registers 0..7
            self.registers[a - MEM_SIZE]
        } else {
            // numbers 32776..65535 are invalid
            fatal!("bad memory value: {} (#{})\n", a, a - MEM_SIZE);
        }
    }
    
    fn get_address(&mut self) -> usize {
        self.get() as usize
    }
    
    fn get_register(&mut self) -> usize {
        let a = self.memory[self.pc] as usize;
        self.pc += 1;
        if (MEM_SIZE <= a) && (a - MEM_SIZE < NUM_REGISTERS) {
            a - MEM_SIZE
        } else {
            fatal!("bad register lvalue: {} (#{})", a, a - MEM_SIZE);
        }
    }
    
    pub fn run(&mut self) {
        loop {
            let op = self.memory[self.pc];
            self.pc += 1;
            match op {
                0 => { // HALT
                    // stop execution and terminate the program
                    break;
                }
                1 => { // SET a b
                    // set register <a> to the value of <b>
                    let a = self.get_register();
                    let b = self.get();
                    self.registers[a] = b;
                }
                2 => { // PUSH a
                    // push <a> onto the stack
                    let a = self.get();
                    self.stack.push(a);
                }
                3 => { // POP a
                    // remove the top element from the stack and write it into <a>; empty stack = error
                    match self.stack.pop() {
                        Some(v) => {
                            let a = self.get_register();
                            self.registers[a] = v;
                        }
                        None => {
                            fatal!("pop from empty stack at address {}", self.pc - 1);
                        }
                    }
                }
                4 => { // EQ a b c
                    // set <a> to 1 if <b> is equal to <c>; set it to 0 otherwise
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = if b == c { 1 } else { 0 };
                }
                5 => { // GT a b c
                    // set <a> to 1 if <b> is greater than <c>; set it to 0 otherwise
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = if b > c { 1 } else { 0 };
                }
                6 => { // JMP a
                    // jump to <a>
                    self.pc = self.get_address()
                }
                7 => { // JT a b
                    // if <a> is nonzero, jump to <b>
                    let a = self.get();
                    let b = self.get_address();
                    if a != 0 {
                        self.pc = b;
                    }
                }
                8 => { // JF a b
                    // if <a> is zero, jump to <b>
                    let a = self.get();
                    let b = self.get_address();
                    if a == 0 {
                        self.pc = b;
                    }
                }
                9 => { // ADD a b c
                    // assign into <a> the sum of <b> and <c> (modulo 32768)
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = (b + c) % BASE;
                }
                10 => { // MULT a b c
                    // store into <a> the product of <b> and <c> (modulo 32768)
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = b.wrapping_mul(c) % BASE;
                }
                11 => { // MOD a b c
                    // store into <a> the remainder of <b> divided by <c>
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = b % c;
                }
                12 => { // AND a b c
                    // stores into <a> the bitwise and of <b> and <c>
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = b & c;
                }
                13 => { // OR a b c
                    // stores into <a> the bitwise or of <b> and <c>
                    let a = self.get_register();
                    let b = self.get();
                    let c = self.get();
                    self.registers[a] = b | c;
                }
                14 => { // NOT a b
                    // stores 15-bit bitwise inverse of <b> in <a>
                    let a = self.get_register();
                    let b = self.get();
                    self.registers[a] = !b & (BASE - 1);
                }
                15 => { // RMEM a b
                    // read memory at address <b> and write it to <a>
                    let a = self.get_register();
                    let b = self.get_address();
                    self.registers[a] = self.memory[b];
                }
                16 => { // WMEM a b
                    // write the value from <b> into memory at address <a>
                    let a = self.get_address();
                    let b = self.get();
                    self.memory[a] = b;
                }
                17 => { // CALL a
                    // write the address of the next instruction to the stack and jump to <a>
                    let a = self.get_address();
                    self.stack.push(self.pc as u16);
                    self.pc = a;
                }
                18 => { // RET
                    // remove the top element from the stack and jump to it; empty stack = halt
                    match self.stack.pop() {
                        Some(v) => { self.pc = v as usize; }
                        None    => { break; }
                    }
                }
                19 => { // OUT a
                    // write the character represented by ascii code <a> to the terminal
                    let a = self.get();
                    print!("{}", a as u8 as char);
                }
                20 => { // IN a
                    // read a character from the terminal and write its ascii code to <a>
                    let a = self.get_register();
                    match std::io::stdin().bytes().next() {
                        Some(Ok(v)) => {
                            let b = (v as u16) % BASE;
                            self.registers[a] = b;
                        }
                        _ => {
                            fatal!("read error");
                        }
                    }
                }
                21 => { // NOOP
                    // no operation
                }
                _ => {
                    fatal!("bad opcode {} at address {}", op, self.pc - 1);
                }
            }
        }
    }
}

fn main() {
    let mut vm = VM::new();
    let program = vec![9, 32768, 32769, 65, 19, 32768];
    vm.init(&program);
    let filename = env::args().nth(1).unwrap_or("challenge.bin".to_owned());
    let filename2 = filename.clone();
    match vm.load(filename) {
        Ok(_) => {}
        Err(_) => { fatal!("cannot read program file {}", filename2); }
    }
    vm.run();
}
