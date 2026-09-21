//! Servicio de ruido continuo. Sus fallos se devuelven al llamador y nunca deben cerrar la UI.
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseKind {
    White,
    Pink,
    Brown,
}

impl NoiseKind {
    pub const ALL: [Self; 3] = [Self::White, Self::Pink, Self::Brown];

    pub fn label(self) -> &'static str {
        match self {
            Self::White => "Blanco",
            Self::Pink => "Rosa",
            Self::Brown => "Marrón",
        }
    }
}

pub struct AudioService {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    kind: NoiseKind,
    volume: f32,
}

/// Reproducción corta para avisos del temporizador.
///
/// Conserva el stream mientras el sonido está activo para que la llamada pueda
/// escucharse incluso si el ruido continuo no está encendido.
pub struct CompletionSound {
    _stream: OutputStream,
    sink: Sink,
}

impl CompletionSound {
    /// Reproduce una llamada sintética inspirada en el silbido del benteveo.
    /// No requiere descargar ni incluir una grabación externa.
    pub fn play_benteveo_call() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
        let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
        sink.set_volume(0.45);
        sink.append(benteveo_call_source());
        Ok(Self {
            _stream: stream,
            sink,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }
}

impl AudioService {
    pub fn new(kind: NoiseKind, volume: u8) -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: None,
            kind,
            volume: normalized_volume(volume),
        })
    }

    pub fn is_playing(&self) -> bool {
        self.sink.is_some()
    }

    pub fn play(&mut self) -> Result<(), String> {
        if self.sink.is_some() {
            return Ok(());
        }
        let sink = Sink::try_new(&self.handle).map_err(|error| error.to_string())?;
        sink.set_volume(self.volume);
        sink.append(noise_source(self.kind));
        self.sink = Some(sink);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = normalized_volume(volume);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn set_kind(&mut self, kind: NoiseKind) -> Result<(), String> {
        if self.kind == kind {
            return Ok(());
        }
        let was_playing = self.is_playing();
        self.stop();
        self.kind = kind;
        if was_playing {
            self.play()?;
        }
        Ok(())
    }
}

fn normalized_volume(volume: u8) -> f32 {
    f32::from(volume.min(100)) / 100.0
}

fn noise_source(kind: NoiseKind) -> impl Source<Item = f32> + Send {
    const SAMPLE_RATE: u32 = 48_000;
    let mut state = 0xA53C_91E7_u32;
    let mut pink = 0.0_f32;
    let mut brown = 0.0_f32;
    let mut samples = Vec::with_capacity(SAMPLE_RATE as usize * 2);
    for _ in 0..SAMPLE_RATE {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let white = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
        pink = pink * 0.98 + white * 0.02;
        brown = (brown + white * 0.02).clamp(-1.0, 1.0);
        let value = match kind {
            NoiseKind::White => white,
            NoiseKind::Pink => pink * 3.5,
            NoiseKind::Brown => brown * 1.8,
        }
        .clamp(-1.0, 1.0)
            * 0.18;
        samples.extend([value, value]);
    }
    SamplesBuffer::new(2, SAMPLE_RATE, samples).repeat_infinite()
}

/// Una frase ascendente de tres silbidos con una repetición suave.
///
/// Se genera como PCM estéreo para mantener el binario autocontenido y evitar
/// que una alarma o un recurso descargado sea necesario para finalizar un foco.
fn benteveo_call_source() -> SamplesBuffer<f32> {
    const SAMPLE_RATE: u32 = 48_000;
    const PHRASE: &[(f32, f32, u32, u32)] = &[
        (1_650.0, 2_000.0, 105, 45),
        (2_050.0, 1_760.0, 120, 55),
        (2_250.0, 2_850.0, 170, 180),
        (1_650.0, 2_000.0, 105, 45),
        (2_050.0, 1_760.0, 120, 55),
        (2_250.0, 2_850.0, 170, 0),
    ];

    let mut samples = Vec::new();
    for &(start_hz, end_hz, length_ms, gap_ms) in PHRASE {
        let frames = (SAMPLE_RATE as u64 * u64::from(length_ms) / 1_000) as usize;
        let attack = (SAMPLE_RATE / 125) as usize; // 8 ms
        let release = (SAMPLE_RATE / 25) as usize; // 40 ms
        let mut phase = 0.0_f32;
        for frame in 0..frames {
            let progress = frame as f32 / frames as f32;
            let frequency = start_hz + (end_hz - start_hz) * progress;
            phase += std::f32::consts::TAU * frequency / SAMPLE_RATE as f32;
            let envelope = if frame < attack {
                frame as f32 / attack as f32
            } else if frame + release > frames {
                (frames - frame) as f32 / release as f32
            } else {
                1.0
            };
            let waveform =
                phase.sin() * 0.72 + (phase * 2.0).sin() * 0.20 + (phase * 3.0).sin() * 0.08;
            let sample = waveform * envelope * 0.32;
            samples.extend([sample, sample]);
        }
        let silent_frames = (SAMPLE_RATE as u64 * u64::from(gap_ms) / 1_000) as usize;
        samples.extend(std::iter::repeat_n(0.0, silent_frames * 2));
    }
    SamplesBuffer::new(2, SAMPLE_RATE, samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benteveo_call_is_stereo_and_finite() {
        let source = benteveo_call_source();
        assert_eq!(source.channels(), 2);
        assert_eq!(source.sample_rate(), 48_000);
        assert!(source.total_duration().is_some());
    }
}
