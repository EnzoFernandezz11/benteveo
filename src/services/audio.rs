//! Servicio de ruido continuo. Sus fallos se devuelven al llamador y nunca deben cerrar la UI.
use std::io::Cursor;

use rodio::{buffer::SamplesBuffer, Decoder, OutputStream, OutputStreamHandle, Sink, Source};

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
    /// Reproduce los primeros cinco segundos de una llamada de benteveo.
    ///
    /// La grabación va embebida en el binario, por lo que la alarma sigue
    /// funcionando sin conexión ni archivos auxiliares.
    pub fn play_benteveo_call() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
        let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
        sink.set_volume(0.45);
        sink.append(benteveo_alarm_source()?);
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
        sink.append(noise_source(self.kind)?);
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

fn noise_source(kind: NoiseKind) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    if matches!(kind, NoiseKind::White) {
        return Ok(Box::new(
            white_noise_source()?
                .convert_samples::<f32>()
                .repeat_infinite(),
        ));
    }

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
            NoiseKind::White => unreachable!("el ruido blanco usa la grabación embebida"),
            NoiseKind::Pink => pink * 3.5,
            NoiseKind::Brown => brown * 1.8,
        }
        .clamp(-1.0, 1.0)
            * 0.18;
        samples.extend([value, value]);
    }
    Ok(Box::new(
        SamplesBuffer::new(2, SAMPLE_RATE, samples).repeat_infinite(),
    ))
}

/// Ruido blanco suave embebido en el binario para que funcione sin conexión.
fn white_noise_source() -> Result<Decoder<Cursor<&'static [u8]>>, String> {
    Decoder::new(Cursor::new(
        include_bytes!("../../themediaguy-soft-soothing-deep-white-noise-378857.mp3").as_slice(),
    ))
    .map_err(|error| error.to_string())
}

fn benteveo_alarm_source() -> Result<Decoder<Cursor<&'static [u8]>>, String> {
    Decoder::new(Cursor::new(
        include_bytes!("../../resources/benteveo-alarm.mp3").as_slice(),
    ))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benteveo_alarm_is_stereo_and_finite() {
        let source = benteveo_alarm_source().expect("el MP3 de alarma debe poder decodificarse");
        assert_eq!(source.channels(), 2);
        assert_eq!(source.sample_rate(), 24_000);
        assert!(source.total_duration().is_some());
    }
}
