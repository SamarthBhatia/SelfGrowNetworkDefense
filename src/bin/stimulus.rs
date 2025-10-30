use morphogenetic_security::stimulus::{StimulusCommand, append_command};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let file_path = args
        .next()
        .ok_or_else(|| "Usage: stimulus <file> <topic> <value> <step>".to_string())?;
    let topic = args
        .next()
        .ok_or_else(|| "Missing <topic> argument".to_string())?;
    let value_str = args
        .next()
        .ok_or_else(|| "Missing <value> argument".to_string())?;
    let step_str = args
        .next()
        .ok_or_else(|| "Missing <step> argument".to_string())?;

    if args.next().is_some() {
        return Err("Too many arguments supplied".to_string());
    }

    let value: f32 = value_str
        .parse()
        .map_err(|_| format!("Invalid <value> `{value_str}`; expected float"))?;
    let step: u32 = step_str
        .parse()
        .map_err(|_| format!("Invalid <step> `{step_str}`; expected unsigned integer"))?;

    let command = StimulusCommand { step, topic, value };

    append_command(PathBuf::from(file_path), &command)
        .map_err(|err| format!("Failed to append stimulus command: {err}"))?;
    println!(
        "Appended stimulus command: step={}, topic={}, value={}",
        command.step, command.topic, command.value
    );
    Ok(())
}
