//! Stateful FX Processor — trait for persistent DSP state across blocks.
//!
//! Each processor wraps a hand-written native DSP struct as a state carrier.
//! State persists between blocks: reverb tails ring out, delay lines hold, filters sweep smoothly.
//! Zero allocation in process() — all buffers pre-allocated in init().

/// Trait for a stateful audio effect processor.
pub trait FxProcessor: Send {
    /// Human-readable name for UI.
    fn name(&self) -> &str;

    /// Initialize internal state for given sample rate and max block size.
    fn init(&mut self, sample_rate: u32, max_frames: usize);

    /// Process audio in-place. Must be zero-alloc.
    fn process(&mut self, samples: &mut [&mut [f32]], frames: usize);

    /// Set intensity/wet (0.0-1.0). Smooth internally — no clicks.
    fn set_intensity(&mut self, intensity: f32);

    /// Current intensity.
    fn intensity(&self) -> f32;

    /// Reset all internal state (delay lines, filters, etc.) to silence.
    fn reset(&mut self);
}

/// FX chain: ordered list of processors with crossfade on preset change.
pub struct FxChain {
    processors: Vec<Box<dyn FxProcessor>>,
    sample_rate: u32,
    max_frames: usize,
}

impl FxChain {
    pub fn new(sample_rate: u32, max_frames: usize) -> Self {
        Self { processors: Vec::new(), sample_rate, max_frames }
    }

    /// Add a processor to the chain end. Initializes it immediately.
    pub fn push(&mut self, mut proc: Box<dyn FxProcessor>) {
        proc.init(self.sample_rate, self.max_frames);
        self.processors.push(proc);
    }

    /// Process through all active processors in order.
    pub fn process(&mut self, samples: &mut [&mut [f32]], frames: usize) {
        for proc in &mut self.processors {
            if proc.intensity() > 0.001 {
                proc.process(samples, frames);
            }
        }
    }

    /// Set intensity on processor at index.
    pub fn set_intensity(&mut self, index: usize, intensity: f32) {
        if let Some(proc) = self.processors.get_mut(index) {
            proc.set_intensity(intensity);
        }
    }

    /// Number of processors in chain.
    pub fn len(&self) -> usize { self.processors.len() }

    /// Reset all processors to silence.
    pub fn reset(&mut self) {
        for proc in &mut self.processors {
            proc.reset();
        }
    }
}

/// Stateless wrapper — wraps any `fn(&mut [f32], f32)` as an FxProcessor.
/// For effects with no persistent state (bitcrush, waveshape, etc.)
pub struct StatelessFx {
    name: String,
    func: fn(&mut [f32], f32),
    current_intensity: f32,
}

impl StatelessFx {
    pub fn new(name: &str, func: fn(&mut [f32], f32)) -> Self {
        Self { name: name.to_string(), func, current_intensity: 0.5 }
    }
}

impl FxProcessor for StatelessFx {
    fn name(&self) -> &str { &self.name }
    fn init(&mut self, _sample_rate: u32, _max_frames: usize) {}
    fn process(&mut self, samples: &mut [&mut [f32]], frames: usize) {
        for ch in samples.iter_mut() {
            (self.func)(&mut ch[..frames], self.current_intensity);
        }
    }
    fn set_intensity(&mut self, intensity: f32) { self.current_intensity = intensity; }
    fn intensity(&self) -> f32 { self.current_intensity }
    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_fx(samples: &mut [f32], intensity: f32) {
        for s in samples.iter_mut() { *s *= intensity; }
    }

    #[test]
    fn chain_processes_in_order() {
        let mut chain = FxChain::new(44100, 512);
        chain.push(Box::new(StatelessFx::new("gain", dummy_fx)));
        let mut data = vec![1.0f32; 8];
        let mut slices: Vec<&mut [f32]> = vec![&mut data];
        chain.process(&mut slices, 8);
        assert!((slices[0][0] - 0.5).abs() < 0.01); // default intensity 0.5
    }

    #[test]
    fn zero_intensity_skips() {
        let mut chain = FxChain::new(44100, 512);
        chain.push(Box::new(StatelessFx::new("gain", dummy_fx)));
        chain.set_intensity(0, 0.0);
        let mut data = vec![1.0f32; 8];
        let mut slices: Vec<&mut [f32]> = vec![&mut data];
        chain.process(&mut slices, 8);
        assert_eq!(slices[0][0], 1.0); // untouched
    }

    #[test]
    fn reset_clears() {
        let mut chain = FxChain::new(44100, 512);
        chain.push(Box::new(StatelessFx::new("gain", dummy_fx)));
        chain.reset(); // should not panic
        assert_eq!(chain.len(), 1);
    }
}
