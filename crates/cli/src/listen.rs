//! `dsk listen`: record a spoken answer from the microphone, transcribe
//! it on this machine, and print the transcript with delivery checks.
//!
//! Analogy: a voice memo app with a court reporter attached. It starts
//! recording when you start talking, stops when you pause (or press
//! Enter), and the reporter (a local Whisper model) writes down what you
//! said, already knowing how to spell Theotokos and hilastērion.
//!
//! Nothing leaves the machine: audio goes from `arecord` (or `parecord`)
//! into memory, then to a WAV file in the cache that is deleted after
//! transcription unless `--keep` is given.

use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use rusqlite::Connection;
use serde::Deserialize;

const RATE: usize = 16_000;
/// Samples per loudness frame (0.1 s).
const FRAME: usize = RATE / 10;

/// The transcriber, run as `python3 -c`. It prints one JSON object:
/// {"text": "...", "duration": 71.2, "segments": [[start, end, text], ...]}.
const TRANSCRIBE_PY: &str = r#"
import json, sys
from faster_whisper import WhisperModel
wav, model_name, prompt = sys.argv[1], sys.argv[2], sys.argv[3]
model = WhisperModel(model_name, device="cpu", compute_type="int8")
segments, info = model.transcribe(wav, language="en", initial_prompt=prompt, vad_filter=False)
segs = [[round(s.start, 2), round(s.end, 2), s.text.strip()] for s in segments]
print(json.dumps({"text": " ".join(s[2] for s in segs).strip(), "duration": round(info.duration, 2), "segments": segs}))
"#;

/// Words Whisper tends to misspell in this vault. Note titles from the
/// index are added after these. The opening "Um, uh" nudges Whisper to
/// keep filler words, which the delivery check counts.
const VOCAB: &[&str] = &[
    "Um, uh, so, you know, I mean.",
    "Theotokos", "theosis", "latreia", "proskynesis", "dulia", "hilasterion", "hilastērion",
    "theologoumenon", "Septuagint", "LXX", "Masoretic", "Orthodox Study Bible", "phronema",
    "ekklesia", "economia", "anathema", "Nicaea", "Chalcedon", "iconoclasm", "Athanasius",
    "Chrysostom", "Palamas", "hesychasm", "Cyprian", "Irenaeus", "Ignatius", "Basil", "Damascene",
    "sola scriptura", "sola fide", "penal substitution", "ancestral sin", "uncreated energies",
    "Eucharist", "Pascha", "Torah", "Hebrews", "Isaiah", "Melchizedek", "filioque", "Dositheus",
];

pub struct Options {
    /// Longest recording, in seconds.
    pub max_secs: f32,
    /// Pause length that ends the recording, in seconds.
    pub silence_secs: f32,
    /// Keep the WAV file instead of deleting it.
    pub keep: bool,
    /// Transcribe this file instead of recording.
    pub file: Option<PathBuf>,
    pub json: bool,
}

pub fn run(conn: Option<&Connection>, opts: &Options) -> Result<()> {
    let cache = didaskalos_core::embed::cache_dir().join("answers");
    std::fs::create_dir_all(&cache)?;

    let (wav, recorded) = match &opts.file {
        Some(f) => (f.clone(), false),
        None => {
            let samples = record(opts)?;
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let path = cache.join(format!("answer_{secs}.wav"));
            write_wav(&path, &samples)?;
            (path, true)
        }
    };

    let result = transcribe(&wav, &prompt(conn));
    if recorded && !opts.keep {
        let _ = std::fs::remove_file(&wav);
    } else if recorded {
        eprintln!("(audio kept at {})", wav.display());
    }
    let t = result?;
    let stats = Delivery::of(&t);

    if opts.json {
        let v = serde_json::json!({
            "text": t.text,
            "duration": t.duration,
            "words": stats.words,
            "words_per_minute": stats.wpm,
            "fillers": stats.fillers,
            "first_sentence": stats.first_sentence,
        });
        println!("{v}");
    } else {
        println!("Transcript:\n{}\n", t.text);
        println!("{}", stats.summary(t.duration));
    }
    Ok(())
}

/// Record until a pause after speech, Enter (when run in a terminal), or
/// the time limit. Returns 16 kHz mono samples.
fn record(opts: &Options) -> Result<Vec<i16>> {
    let mut child = recorder()?;
    let mut out = child.stdout.take().ok_or_else(|| anyhow!("recorder gave no output"))?;

    let stop = Arc::new(AtomicBool::new(false));
    let interactive = std::io::stdin().is_terminal();
    if interactive {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            stop.store(true, Ordering::SeqCst);
        });
    }
    eprintln!(
        "Listening. Speak your answer; it stops after a {:.0}-second pause{}.",
        opts.silence_secs,
        if interactive { " or when you press Enter" } else { "" }
    );

    let mut vad = Vad::new((opts.silence_secs * 10.0) as usize, 200);
    let mut samples: Vec<i16> = Vec::new();
    let mut buf = vec![0u8; FRAME * 2];
    let started = Instant::now();
    let outcome = loop {
        if stop.load(Ordering::SeqCst) {
            break Ok(());
        }
        if started.elapsed().as_secs_f32() > opts.max_secs {
            eprintln!("(stopped at the {:.0}-second limit)", opts.max_secs);
            break Ok(());
        }
        if let Err(e) = out.read_exact(&mut buf) {
            break Err(anyhow!("microphone stopped: {e}"));
        }
        let frame: Vec<i16> = buf.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        samples.extend_from_slice(&frame);
        match vad.push(rms(&frame)) {
            VadState::Waiting if vad.frames > 200 => break Err(anyhow!("no speech heard in 20 seconds; check the microphone")),
            VadState::Done => break Ok(()),
            _ => {}
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    outcome?;
    eprint!("Transcribing… ");
    let _ = std::io::stderr().flush();
    Ok(samples)
}

/// `arecord` (ALSA, on Pop!_OS through PipeWire), else `parecord`.
fn recorder() -> Result<std::process::Child> {
    let tries: [(&str, &[&str]); 2] = [
        ("arecord", &["-q", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "raw"]),
        ("parecord", &["--raw", "--format=s16le", "--rate=16000", "--channels=1"]),
    ];
    for (cmd, args) in tries {
        if let Ok(child) = Command::new(cmd).args(args).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
            return Ok(child);
        }
    }
    bail!("no recorder found: install alsa-utils (arecord) or pulseaudio-utils (parecord)")
}

fn rms(frame: &[i16]) -> f32 {
    let sum: f64 = frame.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / frame.len().max(1) as f64).sqrt() as f32
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum VadState {
    Waiting,
    Speaking,
    Done,
}

/// Voice activity, by loudness: the first half second sets the room's
/// noise floor; speech is anything clearly above it; a long enough quiet
/// stretch after speech ends the recording.
struct Vad {
    frames: usize,
    floor: f32,
    calibration: Vec<f32>,
    loud_run: usize,
    quiet_run: usize,
    quiet_to_stop: usize,
    min_threshold: f32,
    state: VadState,
}

impl Vad {
    fn new(quiet_to_stop: usize, min_threshold: u16) -> Self {
        Self {
            frames: 0,
            floor: 0.0,
            calibration: Vec::new(),
            loud_run: 0,
            quiet_run: 0,
            quiet_to_stop,
            min_threshold: min_threshold as f32,
            state: VadState::Waiting,
        }
    }

    fn push(&mut self, level: f32) -> VadState {
        self.frames += 1;
        if self.calibration.len() < 5 {
            self.calibration.push(level);
            let mut c = self.calibration.clone();
            c.sort_by(f32::total_cmp);
            self.floor = c[c.len() / 2];
            return self.state;
        }
        let loud = level > (self.floor * 2.5).max(self.min_threshold);
        match self.state {
            VadState::Waiting => {
                self.loud_run = if loud { self.loud_run + 1 } else { 0 };
                if self.loud_run >= 2 {
                    self.state = VadState::Speaking;
                }
            }
            VadState::Speaking => {
                self.quiet_run = if loud { 0 } else { self.quiet_run + 1 };
                if self.quiet_run >= self.quiet_to_stop {
                    self.state = VadState::Done;
                }
            }
            VadState::Done => {}
        }
        self.state
    }
}

fn write_wav(path: &Path, samples: &[i16]) -> Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&1u16.to_le_bytes())?; // mono
    f.write_all(&(RATE as u32).to_le_bytes())?;
    f.write_all(&(RATE as u32 * 2).to_le_bytes())?; // byte rate
    f.write_all(&2u16.to_le_bytes())?; // block align
    f.write_all(&16u16.to_le_bytes())?; // bits per sample
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        f.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}

/// The transcription prompt: the fixed vocabulary, then wiki concept
/// titles from the index, kept short (Whisper reads about 220 tokens).
fn prompt(conn: Option<&Connection>) -> String {
    let mut words: Vec<String> = VOCAB.iter().map(|s| s.to_string()).collect();
    if let Some(conn) = conn {
        if let Ok(mut stmt) = conn.prepare("SELECT title FROM notes WHERE kind = 'wiki-concept' ORDER BY title LIMIT 40") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                words.extend(rows.flatten());
            }
        }
    }
    let mut out = String::new();
    for w in words {
        if out.len() + w.len() > 800 {
            break;
        }
        out.push_str(&w);
        out.push_str(", ");
    }
    out.trim_end_matches([',', ' ']).to_string()
}

#[derive(Debug, Deserialize)]
struct Transcript {
    text: String,
    duration: f64,
}

/// Run the transcriber: `$DSK_TRANSCRIBE` if set (any command that takes
/// the WAV path as its last argument and prints the text), otherwise
/// faster-whisper through python3 with `$DSK_WHISPER_MODEL` (default
/// base.en).
fn transcribe(wav: &Path, prompt: &str) -> Result<Transcript> {
    if let Ok(cmd) = std::env::var("DSK_TRANSCRIBE") {
        let mut parts = cmd.split_whitespace();
        let prog = parts.next().ok_or_else(|| anyhow!("$DSK_TRANSCRIBE is empty"))?;
        let out = Command::new(prog).args(parts).arg(wav).output().with_context(|| format!("running {prog}"))?;
        if !out.status.success() {
            bail!("{prog} failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let duration = std::fs::metadata(wav).map(|m| (m.len().saturating_sub(44)) as f64 / (RATE as f64 * 2.0)).unwrap_or(0.0);
        return Ok(Transcript { text, duration });
    }
    let model = std::env::var("DSK_WHISPER_MODEL").unwrap_or_else(|_| "base.en".into());
    let out = Command::new("python3")
        .arg("-c")
        .arg(TRANSCRIBE_PY)
        .arg(wav)
        .arg(&model)
        .arg(prompt)
        .output()
        .context("running python3 for transcription")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("No module named 'faster_whisper'") {
            bail!("faster-whisper is not installed: run `pip install --user faster-whisper` (or set $DSK_TRANSCRIBE)");
        }
        bail!("transcription failed: {}", err.trim());
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let line = line.lines().last().unwrap_or("");
    serde_json::from_str(line).with_context(|| format!("reading transcriber output: {line}"))
}

const FILLERS: &[&str] = &["um", "uh", "er", "ah", "erm", "hmm", "you know", "i mean", "kind of", "sort of", "basically", "like"];

#[derive(Debug)]
struct Delivery {
    words: usize,
    wpm: f64,
    fillers: Vec<(String, usize)>,
    first_sentence: String,
}

impl Delivery {
    fn of(t: &Transcript) -> Self {
        let lower = t.text.to_lowercase();
        let tokens: Vec<String> = lower
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();
        let mut fillers = Vec::new();
        for f in FILLERS {
            let parts: Vec<&str> = f.split(' ').collect();
            let n = tokens.windows(parts.len()).filter(|w| w.iter().zip(&parts).all(|(a, b)| a == b)).count();
            // "like" is often a real word; count it only when it is frequent.
            if n > 0 && (*f != "like" || n >= 3) {
                fillers.push((f.to_string(), n));
            }
        }
        fillers.sort_by(|a, b| b.1.cmp(&a.1));
        let first_sentence = t
            .text
            .split_inclusive(['.', '?', '!'])
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let minutes = (t.duration / 60.0).max(1.0 / 60.0);
        Delivery { words: tokens.len(), wpm: (tokens.len() as f64 / minutes).round(), fillers, first_sentence }
    }

    fn summary(&self, duration: f64) -> String {
        let fillers: usize = self.fillers.iter().map(|(_, n)| n).sum();
        let detail = self.fillers.iter().map(|(f, n)| format!("{f} ×{n}")).collect::<Vec<_>>().join(", ");
        let length = match duration {
            d if d < 30.0 => "short for one turn",
            d if d <= 90.0 => "in the 60–90 s range for one turn",
            _ => "longer than one turn usually allows",
        };
        format!(
            "Delivery: {:.0} s ({length}), {} words, {} per minute, {} filler words{}\nFirst sentence: \"{}\"",
            duration,
            self.words,
            self.wpm,
            fillers,
            if detail.is_empty() { String::new() } else { format!(" ({detail})") },
            self.first_sentence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_waits_for_speech_then_stops_on_a_pause() {
        let mut v = Vad::new(3, 200);
        for _ in 0..5 {
            v.push(50.0); // room noise
        }
        assert_eq!(v.push(60.0), VadState::Waiting);
        v.push(2000.0);
        assert_eq!(v.push(2500.0), VadState::Speaking);
        v.push(40.0);
        v.push(1800.0); // a short breath does not end it
        v.push(40.0);
        v.push(40.0);
        assert_eq!(v.push(40.0), VadState::Done);
    }

    #[test]
    fn delivery_counts_fillers_and_pace() {
        let t = Transcript {
            text: "Um, Orthodoxy does teach that Christ died for us. Uh, you know, the question is the model.".into(),
            duration: 6.0,
        };
        let d = Delivery::of(&t);
        assert_eq!(d.first_sentence, "Um, Orthodoxy does teach that Christ died for us.");
        assert_eq!(d.fillers.iter().map(|(_, n)| n).sum::<usize>(), 3);
        assert_eq!(d.words, 17);
        assert_eq!(d.wpm, 170.0);
    }

    #[test]
    fn wav_header_is_44_bytes() {
        let p = std::env::temp_dir().join(format!("dsk_wav_{}.wav", std::process::id()));
        write_wav(&p, &[0i16; 16_000]).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), 44 + 32_000);
        std::fs::remove_file(p).unwrap();
    }
}
