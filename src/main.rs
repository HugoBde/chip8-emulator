use chip_8::emulator::Emulator;
use std::env::args;

fn main() {
    println!("Welcome to my Chip-8 emulator");
    let args: Vec<String> = args().collect();
    let file;
    if args.len() < 2 {
        println!("No specified ROM!\nAttempting to run \"test_opcode.ch8\"");
        file = "rom/test_opcode.ch8";
    } else {
        println!("Attempting to run \"{}\"", args[1]);
        file = args[1].as_str();
    }
    let mut emu = Emulator::new(file);
    emu.run();
}
