use crate::sound::SquareWave;

use rand::prelude::random;
use sdl2::audio::{AudioDevice, AudioSpecDesired};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::{EventPump, EventSubsystem};

use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

fn shit_pants(pc: usize, opcode: usize) {
    panic!("Incorrect opcode: {:#X} @ {:#X}", opcode, pc);
}

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const SCREEN_HEIGHT: usize = 32;
const SCREEN_WIDTH: usize = 64;

#[allow(dead_code)]
pub struct Emulator {
    pub memory: [u8; 0x1000], // 4096 bytes = 4 kiB programs start at 0x200 or 0x600
    v: [u8; 0x10],            // 16 8-bit registers Vx (V1 = v[1], Vf = v[0xF])
    ireg: usize,
    sound_timer: u8,
    delay_timer: u8,
    pc: usize,
    sp: usize,
    stack: [usize; 0x10],
    display_buffer: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT],
    display_canvas: Canvas<Window>,
    sound_device: AudioDevice<SquareWave>,
    event_pump: EventPump,
    event_subsystem: EventSubsystem,
}

impl Emulator {
    pub fn new(filename: &str) -> Emulator {
        let mut memory: [u8; 0x1000] = [0; 0x1000];

        for i in 0..FONT.len() {
            memory[i] = FONT[i];
        }

        let file = File::open(filename).expect(format!("Error opening file {}", filename).as_str());
        for (i, byte) in file.bytes().enumerate() {
            memory[i + 0x200] = byte.unwrap();
        }

        let sdl_context = sdl2::init().unwrap();
        let window = sdl_context
            .video()
            .unwrap()
            .window(
                "CHIP-8 EMULATOR",
                (SCREEN_WIDTH * 5) as u32,
                (SCREEN_HEIGHT * 5) as u32,
            )
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };

        let sound_device = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &desired_spec, |spec| SquareWave::new(spec))
            .unwrap();

        let event_pump = sdl_context.event_pump().unwrap();

        let event_subsystem = sdl_context.event().unwrap();

        Emulator {
            memory: memory,
            v: [0; 0x10],
            ireg: 0,
            sound_timer: 0,
            delay_timer: 0,
            pc: 0x200,
            sp: 0,
            stack: [0; 0x10],
            display_buffer: [[false; 64]; 32],
            display_canvas: canvas,
            sound_device: sound_device,
            event_pump: event_pump,
            event_subsystem: event_subsystem,
        }
    }

    pub fn run(&mut self) {
        let mut last_tick = Instant::now();
        'main: loop {
            match self.event_pump.poll_event() {
                Some(Event::Quit { .. }) => break 'main,
                Some(Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                }) => break 'main,
                _ => {}
            }
            // Decay timers if last decay occured over 1/60 seconds ago
            if last_tick.elapsed() > Duration::from_millis(1_000 / 60) {
                last_tick = Instant::now();
                self.sound_timer = self.sound_timer.saturating_sub(1);
                if self.sound_timer == 0 {
                    self.sound_device.pause();
                }
                self.delay_timer = self.delay_timer.saturating_sub(1);
            }
            // let mut shit: [u8;512] = [0; 512];
            // std::io::stdin().read(&mut shit);
            let mut opcode: usize = self.memory[self.pc] as usize;
            opcode <<= 8;
            opcode += self.memory[self.pc + 1] as usize;
            self.run_instruction(opcode);
            self.pc += 2;

            std::thread::sleep(Duration::from_millis(1_000 / 600));
        }
    }

    fn display_flip(&mut self) {
        self.display_canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.display_canvas.clear();
        self.display_canvas
            .set_draw_color(Color::RGB(255, 255, 255));
        for row in 0..SCREEN_HEIGHT {
            for col in 0..SCREEN_WIDTH {
                if self.display_buffer[row][col] {
                    self.display_canvas
                        .fill_rect(Rect::new((col * 5) as i32, (row * 5) as i32, 5, 5))
                        .unwrap_or_else(|_| println!("Error presenting display"));
                }
            }
        }
        self.display_canvas.present();
    }

    #[allow(dead_code)]
    fn console_print_display(&self) {
        for row in self.display_buffer {
            for pixel in row {
                if pixel {
                    print!("█");
                } else {
                    print!(".");
                }
            }
            println!("");
        }
        println!("");
    }

    pub fn run_instruction(&mut self, opcode: usize) {
        match opcode {
            0x00E0 => self.clear_display(),
            0x00EE => self.return_subroutine(),
            0x1000..=0x1FFF => self.jump_addr(opcode & 0x0FFF),
            0x2000..=0x2FFF => self.call_addr(opcode & 0x0FFF),
            0x3000..=0x3FFF => self.skip_equal_reg_byte((opcode & 0x0F00) >> 8, opcode & 0x00FF),
            0x4000..=0x4FFF => self.skip_not_equal_reg_byte((opcode & 0xF00) >> 8, opcode & 0x00FF),
            0x5000..=0x5FFF => {
                self.skip_equal_reg_reg((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4)
            }
            0x6000..=0x6FFF => self.set_reg_byte((opcode & 0x0F00) >> 8, opcode & 0x00FF),
            0x7000..=0x7FFF => self.add_reg_byte((opcode & 0x0F00) >> 8, opcode & 0x00FF),
            0x8000..=0x8FFF => match opcode & 0x000F {
                0x0 => self.set_reg_reg((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x1 => self.or((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x2 => self.and((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x3 => self.xor((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x4 => self.add_reg_reg((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x5 => self.sub_reg_reg((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0x6 => self.shift_right((opcode & 0x0F00) >> 8),
                0x7 => self.sub_not_borrow((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4),
                0xE => self.shit_left((opcode & 0x0F00) >> 8),
                _ => shit_pants(self.pc, opcode),
            },
            0x9000..=0x9FFF => {
                self.skip_not_equal_reg_reg((opcode & 0x0F00) >> 8, (opcode & 0x00F0) >> 4)
            }
            0xA000..=0xAFFF => self.set_ireg(opcode & 0x0FFF),
            0xB000..=0xBFFF => self.jump_v0(opcode & 0x0FFF),
            0xC000..=0xCFFF => self.random(opcode & 0x0F00 >> 8, opcode & 0x00FF),
            0xD000..=0xDFFF => self.draw(
                (opcode & 0x0F00) >> 8,
                (opcode & 0x00F0) >> 4,
                opcode & 0x000F,
            ),
            0xE000..=0xEFFF => match opcode & 0x00FF {
                0x009E => self.skip_key_pressed((opcode & 0x0F00) >> 8),
                0x00A1 => self.skip_key_not_pressed((opcode & 0x0F00) >> 8),
                _ => shit_pants(self.pc, opcode),
            },
            0xF000..=0xFFFF => match opcode & 0x00FF {
                0x0007 => self.set_from_dt((opcode & 0x0F00) >> 8),
                0x000A => self.wait_key_press((opcode & 0x0F00) >> 8),
                0x0015 => self.set_dt((opcode & 0x0F00) >> 8),
                0x0018 => self.set_st((opcode & 0x0F00) >> 8),
                0x001E => self.add_ireg_reg((opcode & 0x0F00) >> 8),
                0x0029 => self.set_ireg_font((opcode & 0x0FFF) >> 8),
                0x0033 => self.store_bcd_reg((opcode & 0x0F00) >> 8),
                0x0055 => self.store_regs((opcode & 0x0F00) >> 8),
                0x0065 => self.read_regs((opcode & 0x0F00) >> 8),
                _ => shit_pants(self.pc, opcode),
            },
            _ => shit_pants(self.pc, opcode),
        }
    }

    fn clear_display(&mut self) {
        println!("{:#X}: CLS", self.pc);
        self.display_buffer = [[false; 64]; 32];
        self.display_flip();
    }

    fn return_subroutine(&mut self) {
        println!("{:#X}: RET {}", self.pc, self.sp);
        self.pc = self.stack[self.sp];
        self.sp -= 1;
    }

    fn jump_addr(&mut self, nnn: usize) {
        println!("{:#X}: JMP {:#X}", self.pc, nnn);
        self.pc = nnn - 2;
    }

    fn call_addr(&mut self, nnn: usize) {
        println!("{:#X}: CALL {:#X} {}", self.pc, nnn, self.sp);
        self.sp += 1;
        self.stack[self.sp] = self.pc;
        self.pc = nnn - 2;
    }

    fn skip_equal_reg_byte(&mut self, x: usize, byte: usize) {
        println!("{:#X}: SE V{:#X}, {:#X}", self.pc, x, byte);
        if self.v[x] == byte as u8 {
            self.pc += 2; // might need to change how this works depending on the implementation of the stack pointer / procram counter
        }
    }

    fn skip_not_equal_reg_byte(&mut self, x: usize, byte: usize) {
        println!("{:#X}: SNE V{:#X}, {:#X}", self.pc, x, byte);
        if self.v[x] != byte as u8 {
            self.pc += 2;
        }
    }

    fn skip_equal_reg_reg(&mut self, x: usize, y: usize) {
        println!("{:#X}: SE V{:#X}, V{:#X}", self.pc, x, y);
        if self.v[x] == self.v[y] {
            self.pc += 2;
        }
    }

    fn set_reg_byte(&mut self, x: usize, byte: usize) {
        println!("{:#X}: LD V{:#X}, {:#X}", self.pc, x, byte);
        self.v[x] = byte as u8;
    }

    fn add_reg_byte(&mut self, x: usize, byte: usize) {
        println!("{:#X}: ADD V{:#X}, {:#X}", self.pc, x, byte);
        self.v[x] = self.v[x].wrapping_add(byte as u8);
    }

    fn set_reg_reg(&mut self, x: usize, y: usize) {
        println!("{:#X}: LD V{:#X}, V{:#X}", self.pc, x, y);
        self.v[x] = self.v[y];
    }

    fn or(&mut self, x: usize, y: usize) {
        println!("{:#X}: OR V{:#X}, V{:#X}", self.pc, x, y);
        self.v[x] |= self.v[y];
    }

    fn and(&mut self, x: usize, y: usize) {
        println!("{:#X}: AND V{:#X}, V{:#X}", self.pc, x, y);
        self.v[x] &= self.v[y];
    }

    fn xor(&mut self, x: usize, y: usize) {
        println!("{:#X}: XOR V{:#X}, V{:#X}", self.pc, x, y);
        self.v[x] ^= self.v[y];
    }

    fn add_reg_reg(&mut self, x: usize, y: usize) {
        println!("{:#X}: ADD V{:#X}, V{:#X}", self.pc, x, y);
        let total = self.v[x] as usize + self.v[y] as usize;
        if total > 255 {
            self.v[x] = total as u8 & 0xFF;
            self.v[0xF] = 1;
        } else {
            self.v[x] = total as u8;
        }
    }

    fn sub_reg_reg(&mut self, x: usize, y: usize) {
        println!("{:#X}: SUB V{:#X}, V{:#X}", self.pc, x, y);
        if self.v[x] >= self.v[y] {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
        self.v[x] = self.v[x].wrapping_sub(self.v[y]);
    }

    fn shift_right(&mut self, x: usize) {
        println!("{:#X}: SHR V{:#X}", self.pc, x);
        self.v[0xF] = self.v[x] & 0x1;
        self.v[x] >>= 1;
    }

    fn sub_not_borrow(&mut self, x: usize, y: usize) {
        println!("{:#X}: SUBN V{:#X}, V{:#X}", self.pc, x, y);
        if self.v[y] >= self.v[x] {
            self.v[0xF] = 1;
        } else {
            self.v[0xF] = 0;
        }
        self.v[x] = self.v[y].wrapping_sub(self.v[x]);
    }

    fn shit_left(&mut self, x: usize) {
        println!("{:#X}: SHL V{:#X}", self.pc, x);
        self.v[0xF] = (self.v[x] & 0x80) >> 7;
        self.v[x] <<= 1;
    }

    fn skip_not_equal_reg_reg(&mut self, x: usize, y: usize) {
        println!("{:#X}: SNE V{:#X}, V{:#X}", self.pc, x, y);
        if self.v[x] != self.v[y] {
            self.pc += 2;
        }
    }

    fn set_ireg(&mut self, nnn: usize) {
        println!("{:#X}: LD I, {:#X}", self.pc, nnn);
        self.ireg = nnn;
    }

    fn jump_v0(&mut self, nnn: usize) {
        println!("{:#X}: JMP V0, {:#X}", self.pc, nnn);
        self.pc = self.v[0] as usize + nnn - 2;
    }

    fn random(&mut self, x: usize, byte: usize) {
        println!("{:#X}: RND V{:#X}, {:#X}", self.pc, x, byte);
        self.v[x] = random::<u8>() & byte as u8;
    }

    fn draw(&mut self, x: usize, y: usize, n: usize) {
        println!("{:#X}: DRW V{:#X}, V{:#X}, {:#X}", self.pc, x, y, n);
        self.v[0xF] = 0;
        for offset in 0..n {
            let row = (self.v[y] + offset as u8) % 32;
            let byte = self.memory[self.ireg + offset];

            for i in 0..8 {
                let col = (self.v[x] + i) % 64;
                let current = &mut self.display_buffer[row as usize][col as usize];
                if byte >> (7 - i) & 1 == 1 {
                    if *current {
                        self.v[0xF] = 1;
                        *current = false;
                    } else {
                        *current = true;
                    }
                }
            }
        }
        self.display_flip();
    }

    fn skip_key_pressed(&mut self, x: usize) {
        println!("{:#X}: SKP V{:#X}", self.pc, x);
        let event = self.event_pump.poll_event();
        match event {
            Some(Event::KeyDown {
                keycode: Some(keycode),
                ..
            }) => {
                if Some(self.v[x]) == interpret_key(keycode) {
                    self.pc += 2;
                }
            }
            _ => {}
        }
    }

    fn skip_key_not_pressed(&mut self, x: usize) {
        println!("{:#X}: SKNP V{:#X}", self.pc, x);
        let event = self.event_pump.poll_event();
        match event {
            Some(Event::KeyDown {
                keycode: Some(keycode),
                ..
            }) => {
                if Some(self.v[x]) == interpret_key(keycode) {
                    self.pc -= 2;
                }
            }
            _ => {}
        }
        self.pc += 2;
    }

    fn set_from_dt(&mut self, x: usize) {
        println!("{:#X}: LD V{:#X}, DT", self.pc, x);
        self.v[x] = self.delay_timer;
    }

    fn wait_key_press(&mut self, x: usize) {
        println!("{:#X}: LD V{:#X}, K", self.pc, x);
        for event in self.event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = interpret_key(keycode) {
                        self.v[x] = key;
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn set_dt(&mut self, x: usize) {
        println!("{:#X}: LD DT, V{:#X}", self.pc, x);
        self.delay_timer = self.v[x];
    }

    fn set_st(&mut self, x: usize) {
        println!("{:#X}: LD ST, V{:#X}", self.pc, x);
        self.sound_timer = self.v[x];
        if self.sound_timer > 0 {
            self.sound_device.resume();
        }
    }

    fn add_ireg_reg(&mut self, x: usize) {
        println!("{:#X}: ADD I, V{:#X}", self.pc, x);
        self.ireg += self.v[x] as usize;
    }

    fn set_ireg_font(&mut self, x: usize) {
        println!("{:#X}: LD F, V{:#X}", self.pc, x);
        self.ireg = self.v[x] as usize * 5;
    }

    fn store_bcd_reg(&mut self, x: usize) {
        println!("{:#X}: LD B, V{:#X}", self.pc, x);
        self.memory[self.ireg] = self.v[x] / 100;
        self.memory[self.ireg + 1] = self.v[x] % 100 / 10;
        self.memory[self.ireg + 2] = self.v[x] % 10;
    }

    fn store_regs(&mut self, x: usize) {
        println!("{:#X}: LD [I], V{:#X}", self.pc, x);
        for i in 0..=x {
            self.memory[self.ireg + i] = self.v[i];
        }
    }

    fn read_regs(&mut self, x: usize) {
        println!("{:#X}: LD V{:#X}, [I]", self.pc, x);
        for i in 0..=x {
            self.v[i] = self.memory[self.ireg + i];
        }
    }
}

fn interpret_key(keycode: Keycode) -> Option<u8> {
    match keycode {
        Keycode::Num0 => Some(0x0),
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0x4),
        Keycode::Num5 => Some(0x5),
        Keycode::Num6 => Some(0x6),
        Keycode::Num7 => Some(0x7),
        Keycode::Num8 => Some(0x8),
        Keycode::Num9 => Some(0x9),
        Keycode::A => Some(0xA),
        Keycode::B => Some(0xB),
        Keycode::C => Some(0xC),
        Keycode::D => Some(0xD),
        Keycode::E => Some(0xE),
        Keycode::F => Some(0xF),
        _ => None,
    }
}
