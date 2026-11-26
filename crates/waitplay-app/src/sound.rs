//! Completion sounds. The built-in sounds are synthesized here (so there is nothing to license and nothing to ship); the user's own
//! file lives in the app-support `sounds` folder. Playing never blocks the UI thread and never panics: a sound that cannot play
//! is simply silent.

use std::path::{Path, PathBuf};
use waitplay_core::paths;
use waitplay_core::settings::{valid_custom_sound_file, Settings, SOUND_NAMES};

const RATE: u32 = 22_050;

/// What happened; the same sound is pitched and shortened to tell the cues apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Completed,
    AllDone,
    Started,
    Opened,
    Error,
}

impl Cue {
    fn file_tag(self) -> &'static str {
        match self {
            Cue::Completed | Cue::AllDone => "done",
            Cue::Started => "start",
            Cue::Opened => "open",
            Cue::Error => "error",
        }
    }
}

pub fn custom_dir() -> PathBuf {
    paths::app_support_dir().join("sounds")
}

fn tone(buf: &mut [f32], start: f32, dur: f32, f0: f32, f1: f32, shape: Shape, gain: f32) {
    let n = (dur * RATE as f32) as usize;
    let s0 = (start * RATE as f32) as usize;
    let mut phase = 0f32;
    for i in 0..n {
        let Some(out) = buf.get_mut(s0 + i) else { break };
        let t = i as f32 / n as f32;
        let f = f0 + (f1 - f0) * t;
        phase += f / RATE as f32;
        let p = phase.fract();
        let wave = match shape {
            Shape::Sine => (p * std::f32::consts::TAU).sin(),
            Shape::Square => if p < 0.5 { 0.6 } else { -0.6 },
            Shape::Bell => (p * std::f32::consts::TAU).sin() + 0.45 * (p * 2.76 * std::f32::consts::TAU).sin() + 0.2 * (p * 5.4 * std::f32::consts::TAU).sin(),
        };
        // short attack (no click), exponential decay
        let attack = (i as f32 / (0.006 * RATE as f32)).min(1.0);
        let decay = (-4.5 * t).exp() * (1.0 - t).clamp(0.0, 1.0).sqrt();
        *out += wave * attack * decay * gain;
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Sine,
    Square,
    Bell,
}

/// 16-bit mono WAV of `name` for `cue` at `volume` (0 to 100). Unknown names fall back to the gentle sound.
pub fn render(name: &str, cue: Cue, volume: u32) -> Vec<u8> {
    let pitch = match cue {
        Cue::Started | Cue::Opened => 1.26,
        _ => 1.0,
    };
    let mut buf = vec![0f32; (RATE as f32 * 1.4) as usize];
    if cue == Cue::Error {
        tone(&mut buf, 0.0, 0.18, 330.0, 310.0, Shape::Sine, 0.5);
        tone(&mut buf, 0.2, 0.32, 247.0, 220.0, Shape::Sine, 0.5);
    } else {
        let f = |hz: f32| hz * pitch;
        match name {
            "soft" => {
                tone(&mut buf, 0.0, 0.32, f(523.0), f(523.0), Shape::Sine, 0.45);
                tone(&mut buf, 0.14, 0.45, f(659.0), f(659.0), Shape::Sine, 0.45);
            }
            "chime" => {
                tone(&mut buf, 0.0, 0.9, f(880.0), f(880.0), Shape::Bell, 0.3);
                tone(&mut buf, 0.18, 0.9, f(1320.0), f(1320.0), Shape::Bell, 0.2);
            }
            "retro" => {
                for (i, hz) in [392.0, 523.0, 659.0, 784.0].iter().enumerate() {
                    tone(&mut buf, i as f32 * 0.07, 0.09, f(*hz), f(*hz), Shape::Square, 0.28);
                }
            }
            "futuristic" => {
                tone(&mut buf, 0.0, 0.35, f(420.0), f(1260.0), Shape::Sine, 0.4);
                tone(&mut buf, 0.12, 0.45, f(630.0), f(1890.0), Shape::Sine, 0.2);
            }
            // "classic" has no synthesized form of its own on Windows: the system sound plays on a Mac, this stands in elsewhere
            "classic" => {
                tone(&mut buf, 0.0, 0.6, f(784.0), f(784.0), Shape::Bell, 0.3);
            }
            _ => {
                for (i, hz) in [523.0, 659.0, 784.0].iter().enumerate() {
                    tone(&mut buf, i as f32 * 0.11, 0.55, f(*hz), f(*hz), Shape::Sine, 0.32);
                }
            }
        }
    }
    let gain = (volume.min(100) as f32 / 100.0).powf(1.5) * 0.9;
    let samples: Vec<i16> = trim_silence(&buf).iter().map(|s| (s * gain).clamp(-1.0, 1.0)).map(|s| (s * 32_767.0) as i16).collect();
    wav(&samples)
}

fn trim_silence(buf: &[f32]) -> &[f32] {
    let end = buf.iter().rposition(|s| s.abs() > 0.0008).map_or(0, |i| i + 1);
    &buf[..end]
}

fn wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Largest sound file accepted from the user.
const MAX_CUSTOM_BYTES: u64 = 5 * 1024 * 1024;

/// Copies the user's sound file into `dir` as `custom.<ext>` (replacing the previous one) and returns that file name. Only known
/// audio extensions and small files are accepted; the original is never touched.
pub fn import_custom(src: &Path, dir: &Path) -> Result<String, String> {
    let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if !valid_custom_sound_file(&format!("custom.{}", ext)) || ext.is_empty() {
        return Err("That file type is not supported. Use WAV, AIFF, CAF, MP3 or M4A.".into());
    }
    let meta = std::fs::metadata(src).map_err(|e| format!("The file could not be read: {}", e))?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_CUSTOM_BYTES {
        return Err("Choose an audio file smaller than 5 MB.".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    remove_custom_in(dir);
    let name = format!("custom.{}", ext);
    std::fs::copy(src, dir.join(&name)).map_err(|e| format!("The file could not be copied: {}", e))?;
    Ok(name)
}

pub fn remove_custom() {
    remove_custom_in(&custom_dir());
}

fn remove_custom_in(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_str().is_some_and(|n| n.starts_with("custom.")) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// What to play for `cue` under `settings`.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    File(PathBuf),
    /// A system sound file (macOS only): played as is, volume through the player.
    System(PathBuf),
    Synth(String),
}

fn resolve(s: &Settings) -> Source {
    if s.sound_name == "custom" && valid_custom_sound_file(&s.sound_custom_file) && !s.sound_custom_file.is_empty() {
        let p = custom_dir().join(&s.sound_custom_file);
        if p.is_file() {
            return Source::File(p);
        }
    }
    if s.sound_name == "classic" && cfg!(target_os = "macos") {
        let p = PathBuf::from("/System/Library/Sounds/Glass.aiff");
        if p.is_file() {
            return Source::System(p);
        }
    }
    let name = if SOUND_NAMES.contains(&s.sound_name.as_str()) { s.sound_name.clone() } else { "gentle".to_string() };
    Source::Synth(name)
}

/// Plays `cue` once, if sounds are on. Returns at once.
pub fn play(s: &Settings, cue: Cue) {
    if s.sound_muted || s.sound_volume == 0 {
        return;
    }
    play_unmuted(s, cue);
}

/// The "Preview" button: plays even while muted (the user asked to hear it), at the chosen volume.
pub fn preview(s: &Settings) {
    play_unmuted(s, Cue::Completed);
}

fn play_unmuted(s: &Settings, cue: Cue) {
    let source = resolve(s);
    let volume = s.sound_volume.min(100);
    let _ = std::thread::Builder::new().name("wp-sound".into()).spawn(move || match source {
        Source::File(p) | Source::System(p) => play_file(&p, volume),
        Source::Synth(name) => {
            let dir = std::env::temp_dir().join("waitplay-sounds");
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join(format!("{}-{}-{}.wav", name, cue.file_tag(), volume));
            if !file.is_file() && std::fs::write(&file, render(&name, cue, volume)).is_err() {
                return;
            }
            // the volume is already in the samples
            play_file(&file, 100)
        }
    });
}

#[cfg(target_os = "macos")]
fn play_file(path: &Path, volume: u32) {
    let _ = std::process::Command::new("/usr/bin/afplay").arg("-v").arg(format!("{:.2}", volume as f32 / 100.0)).arg(path).status();
}

#[cfg(windows)]
fn play_file(path: &Path, _volume: u32) {
    use std::os::windows::process::CommandExt;
    // only WAV plays through SoundPlayer; the settings page offers other formats on a Mac only
    if path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() != Some("wav") {
        return;
    }
    let script = format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path.display().to_string().replace('\'', "''"));
    let _ = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script]).creation_flags(0x0800_0000).status();
}

#[cfg(not(any(target_os = "macos", windows)))]
fn play_file(_path: &Path, _volume: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }

    #[test]
    fn every_sound_and_cue_renders_a_valid_wav_of_sensible_length() {
        for name in SOUND_NAMES.iter().chain(["unknown"].iter()) {
            for cue in [Cue::Completed, Cue::AllDone, Cue::Started, Cue::Opened, Cue::Error] {
                let w = render(name, cue, 70);
                assert_eq!(&w[..4], b"RIFF");
                assert_eq!(&w[8..16], b"WAVEfmt ");
                let data = u32_at(&w, 40) as usize;
                assert_eq!(w.len(), 44 + data, "{name} {cue:?}");
                let secs = data as f32 / 2.0 / RATE as f32;
                assert!((0.15..1.4).contains(&secs), "{name} {cue:?}: {secs}s");
                assert!(w[44..].chunks(2).any(|c| i16::from_le_bytes([c[0], c[1]]).abs() > 500), "{name} {cue:?} is silent");
            }
        }
    }

    #[test]
    fn volume_scales_the_samples() {
        let peak = |v: u32| render("chime", Cue::Completed, v)[44..].chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]]).abs()).max().unwrap_or(0);
        assert!(peak(100) > peak(50) && peak(50) > peak(10));
        assert_eq!(peak(0), 0);
    }

    #[test]
    fn custom_without_a_file_falls_back_to_a_built_in_sound() {
        let mut s = Settings::default();
        s.sound_name = "custom".into();
        assert_eq!(resolve(&s), Source::Synth("gentle".into()), "no custom file yet");
        s.sound_name = "retro".into();
        assert_eq!(resolve(&s), Source::Synth("retro".into()));
    }

    #[test]
    fn importing_a_custom_sound_checks_type_and_size_and_replaces_the_old_one() {
        let base = std::env::temp_dir().join(format!("wp-sound-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("sounds");
        std::fs::create_dir_all(&base).unwrap();
        let wav = base.join("my ding.WAV");
        std::fs::write(&wav, render("chime", Cue::Completed, 50)).unwrap();
        assert_eq!(import_custom(&wav, &dir).unwrap(), "custom.wav");
        let mp3 = base.join("x.mp3");
        std::fs::write(&mp3, b"ID3").unwrap();
        assert_eq!(import_custom(&mp3, &dir).unwrap(), "custom.mp3");
        assert!(!dir.join("custom.wav").exists(), "the previous custom sound is gone");
        let exe = base.join("evil.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert!(import_custom(&exe, &dir).is_err());
        let big = base.join("big.wav");
        std::fs::write(&big, vec![0u8; 5 * 1024 * 1024 + 1]).unwrap();
        assert!(import_custom(&big, &dir).is_err());
        assert!(import_custom(&base.join("missing.wav"), &dir).is_err());
        remove_custom_in(&dir);
        assert!(!dir.join("custom.mp3").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
