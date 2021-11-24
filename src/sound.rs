use sdl2::audio::{AudioCallback, AudioSpec};

pub struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl SquareWave {
    pub fn new(spec: AudioSpec) -> SquareWave {
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        }
    }
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}
