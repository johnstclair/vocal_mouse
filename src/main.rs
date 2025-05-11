use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SupportedStreamConfig};
use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{Button, Coordinate, Direction::Click, Enigo, Mouse, Settings};
use pitch_detector::{
    core::NoteName,
    note::{NoteDetectionResult, detect_note},
    pitch::HannedFftDetector,
};
use serde::Deserialize;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug, Deserialize)]
struct UserConfig {
    mode: String,
    click_threshold: i32,
    default_threshold: i32,

    default_power: i32,
    power_multiplier: f32,

    vol_multiplier: f32,
}

fn main() -> Result<(), anyhow::Error> {
    let content = fs::read_to_string("settings.toml")?;
    let user_config: UserConfig = toml::from_str(&content)?;
    println!("{:#?}", user_config);

    let mut enigo: Enigo = Enigo::new(&Settings::default()).unwrap();

    let host = cpal::default_host();

    #[cfg(any(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    )),))]
    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = host.default_input_device().expect("No input device");

    println!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");
    println!("Default input config: {:?}", config);

    let config_clone = config.clone();

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| detect_pitch::<i8>(data, &config_clone, &mut enigo, &user_config),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| detect_pitch::<i16>(data, &config_clone, &mut enigo, &user_config),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| detect_pitch::<i32>(data, &config_clone, &mut enigo, &user_config),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| detect_pitch::<f32>(data, &config_clone, &mut enigo, &user_config),
            err_fn,
            None,
        )?,
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )));
        }
    };

    stream.play()?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(120));
    }
}

fn calculate_rms(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut sum_of_squares = 0.0;
    for s in data {
        sum_of_squares += s * s;
    }

    let mean_square = sum_of_squares / data.len() as f64;
    mean_square.sqrt() * 1300.0
}

fn detect_pitch<T>(
    input: &[T],
    config: &SupportedStreamConfig,
    cursor: &mut Enigo,
    user_config: &UserConfig,
) where
    T: Sample + Into<f64>,
{
    let device_state = DeviceState::new();

    let input: Vec<f64> = input.iter().map(|v| (*v).into()).collect();

    let mut detector = HannedFftDetector::default();

    let error_free_note = catch_unwind(AssertUnwindSafe(|| {
        detect_note(&input, &mut detector, config.sample_rate().0 as f64)
    }))
    .ok()
    .flatten();

    if let Some(note) = error_free_note {
        let vol = (calculate_rms(&input) as f32 * user_config.vol_multiplier) as i32;

        if vol > user_config.default_threshold {
            'outer: {
                let mut power = ((vol - user_config.default_threshold) as f32
                    * user_config.power_multiplier) as i32
                    + user_config.default_power;

                if device_state.get_keys().contains(&Keycode::LShift) {
                    power *= 2;
                } else if device_state.get_keys().contains(&Keycode::LControl) {
                    power /= 2;
                }

                println!(
                    "note {a}, octave {c}, vol {b}, power {d}",
                    a = note.note_name,
                    b = vol,
                    c = note.octave,
                    d = power,
                );
                if vol > user_config.click_threshold {
                    // check if the vol enough to click
                    if note.octave > 4 {
                        let _ = cursor.button(Button::Right, Click);
                    } else {
                        let _ = cursor.button(Button::Left, Click);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    break 'outer; // break to avoid moving mouse
                }
                match user_config.mode.as_str() {
                    "std" => standard_mouse_behavior(note, cursor, power),
                    "freq" => freq_mouse_behavior(note, cursor, power),
                    "adv" => adv_mouse_behavior(note, cursor, power),
                    _ => {
                        println!("configured mode is not valid");
                        ()
                    }
                }
            }
        }
    }

    drop(device_state)
}

fn standard_mouse_behavior(note: NoteDetectionResult, enigo: &mut Enigo, power: i32) {
    let _ = match note.note_name {
        NoteName::A => enigo.move_mouse(-power, 0, Coordinate::Rel), // left
        NoteName::ASharp => enigo.move_mouse(-power, 0, Coordinate::Rel),
        NoteName::B => enigo.move_mouse(-power, 0, Coordinate::Rel),

        NoteName::C => enigo.move_mouse(0, -power, Coordinate::Rel), // up
        NoteName::CSharp => enigo.move_mouse(0, -power, Coordinate::Rel),
        NoteName::D => enigo.move_mouse(0, -power, Coordinate::Rel),

        NoteName::DSharp => enigo.move_mouse(power, 0, Coordinate::Rel), // right
        NoteName::E => enigo.move_mouse(power, 0, Coordinate::Rel),
        NoteName::F => enigo.move_mouse(power, 0, Coordinate::Rel),

        NoteName::FSharp => enigo.move_mouse(0, power, Coordinate::Rel), // down
        NoteName::G => enigo.move_mouse(0, power, Coordinate::Rel),
        NoteName::GSharp => enigo.move_mouse(0, power, Coordinate::Rel),
    };
}

fn adv_mouse_behavior(note: NoteDetectionResult, enigo: &mut Enigo, power: i32) {
    let _ = match note.note_name {
        NoteName::A => enigo.move_mouse(-power, 0, Coordinate::Rel), // left
        NoteName::ASharp => enigo.move_mouse(-power, 0, Coordinate::Rel),

        NoteName::B => enigo.move_mouse(-power, -power, Coordinate::Rel), // left up

        NoteName::C => enigo.move_mouse(0, -power, Coordinate::Rel), // up
        NoteName::CSharp => enigo.move_mouse(0, -power, Coordinate::Rel),

        NoteName::D => enigo.move_mouse(power, power, Coordinate::Rel), // up right

        NoteName::DSharp => enigo.move_mouse(power, 0, Coordinate::Rel), // right
        NoteName::E => enigo.move_mouse(power, 0, Coordinate::Rel),

        NoteName::F => enigo.move_mouse(power, power, Coordinate::Rel), // right down

        NoteName::FSharp => enigo.move_mouse(0, power, Coordinate::Rel), // down
        NoteName::G => enigo.move_mouse(0, power, Coordinate::Rel),

        NoteName::GSharp => enigo.move_mouse(-power, power, Coordinate::Rel), // down left
    };
}

fn freq_mouse_behavior(note: NoteDetectionResult, enigo: &mut Enigo, power: i32) {
    let _ = match note.actual_freq {
        // default move behavior
        0.0..230.0 => enigo.move_mouse(0, power, Coordinate::Rel), // up
        230.0..310.0 => enigo.move_mouse(-power, 0, Coordinate::Rel), // left
        310.0..400.0 => enigo.move_mouse(power, 0, Coordinate::Rel), // right
        400.0..1000.0 => enigo.move_mouse(0, -power, Coordinate::Rel), // down
        _ => Ok(()),
    };
}
