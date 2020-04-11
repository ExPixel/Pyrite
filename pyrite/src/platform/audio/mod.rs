use miniaudio::{
    Device, DeviceConfig, DeviceType, Format, FramesMut, Waveform, WaveformConfig, WaveformType,
};
use pyrite_gba::audio::{NoiseState, SquareWaveState, WaveOutputState};
use pyrite_gba::GbaAudioOutput;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Abstraction used to output sound.
pub struct PlatformAudio {
    device: Device,
    control: Arc<GbaAudioPlaybackControl>,
}

impl PlatformAudio {
    const DEVICE_FORMAT: Format = Format::F32;
    const DEVICE_CHANNELS: u32 = 2;
    const DEVICE_SAMPLE_RATE: u32 = miniaudio::SAMPLE_RATE_44100;

    pub fn new() -> PlatformAudio {
        let mut device_config = DeviceConfig::new(DeviceType::Playback);
        device_config.playback_mut().set_format(Self::DEVICE_FORMAT);
        device_config
            .playback_mut()
            .set_channels(Self::DEVICE_CHANNELS);
        device_config.set_sample_rate(Self::DEVICE_SAMPLE_RATE);

        let mut gba_playback = GbaAudioPlayback::new();
        let control = Arc::clone(&gba_playback.control);
        device_config.set_data_callback(move |_device, output, _input| {
            gba_playback.output_frames(output);
        });

        device_config.set_stop_callback(|_device| {
            log::info!("stopped audio device");
        });

        let device = Device::new(None, &device_config).expect("failed to open playback device");
        device.start().expect("failed to start playback device");

        log::info!("started audio device");

        PlatformAudio {
            device: device,
            control: control,
        }
    }

    pub fn init(&mut self) {
        /* NOP */
    }

    pub fn set_paused(&mut self, _paused: bool) {
        // TODO
    }

    /// Push some samples to be played.
    pub fn push_samples(&mut self, _samples: &[u16]) {
        // TODO
    }
}

impl GbaAudioOutput for PlatformAudio {
    fn set_tone_sweep_state(&mut self, state: SquareWaveState) {
        self.control
            .channel0_state
            .store(state.value, Ordering::Release);
    }
    fn set_tone_state(&mut self, _state: SquareWaveState) {
        /* NOP */
    }
    fn set_wave_output_state(&mut self, _state: WaveOutputState) {
        /* NOP */
    }
    fn set_noise_state(&mut self, _state: NoiseState) {
        /* NOP */
    }

    fn play_samples(&mut self) {
        /* NOP */
    }
}

#[derive(Default)]
pub struct GbaAudioPlaybackControl {
    channel0_state: AtomicU32,
    channel1_state: AtomicU32,
}

#[derive(Clone)]
pub struct GbaAudioPlayback {
    square_wave_0: Waveform,
    square_wave_1: Waveform,
    control: Arc<GbaAudioPlaybackControl>,
}

impl GbaAudioPlayback {
    pub fn new() -> GbaAudioPlayback {
        let square_wave_config = WaveformConfig::new(
            PlatformAudio::DEVICE_FORMAT,
            PlatformAudio::DEVICE_CHANNELS,
            PlatformAudio::DEVICE_SAMPLE_RATE,
            WaveformType::Square,
            0.0,
            0.1,
        );

        GbaAudioPlayback {
            square_wave_0: Waveform::new(&square_wave_config),
            square_wave_1: Waveform::new(&square_wave_config),
            control: Arc::new(GbaAudioPlaybackControl::default()),
        }
    }

    pub fn output_frames(&mut self, output: &mut FramesMut) {
        let channel0_state =
            SquareWaveState::wrap(self.control.channel0_state.load(Ordering::Acquire));
        // self.square_wave_0.set_frequency(channel0_state.frequency());
        self.square_wave_0.read_pcm_frames(output);
    }
}