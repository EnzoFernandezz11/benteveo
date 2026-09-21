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
