use morphogenetic_security::stimulus::{StimulusCommand, append_command};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(err) = run_with_args(args) {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run_with_args(args: Vec<String>) -> Result<(), String> {
    let mut args_iter = args.into_iter();
    let file_path = args_iter
        .next()
        .ok_or_else(|| "Usage: stimulus <file> <topic> <value> <step>".to_string())?;
    let topic = args_iter
        .next()
        .ok_or_else(|| "Missing <topic> argument".to_string())?;
    let value_str = args_iter
        .next()
        .ok_or_else(|| "Missing <value> argument".to_string())?;
    let step_str = args_iter
        .next()
        .ok_or_else(|| "Missing <step> argument".to_string())?;

    if args_iter.next().is_some() {
        return Err("Too many arguments supplied".to_string());
    }

    let value: f32 = value_str
        .parse()
        .map_err(|_| format!("Invalid <value> `{value_str}`; expected float"))?;
    let step: u32 = step_str
        .parse()
        .map_err(|_| format!("Invalid <step> `{step_str}`; expected unsigned integer"))?;

    let command = StimulusCommand {
        step,
        topic: topic.clone(), // Clone for print
        value,
        target: None,
        source: None,
        duration: 1,
    };

    append_command(PathBuf::from(file_path), &command)
        .map_err(|err| format!("Failed to append stimulus command: {err}"))?;
    println!(
        "Appended stimulus command: step={}, topic={}, value={}",
        step, topic, value
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{read_to_string, remove_file};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_run_with_valid_args() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let path = std::env::temp_dir().join(format!("cli_stimulus_test_{timestamp}.jsonl"));
        let path_str = path.to_str().unwrap().to_string();

        let args = vec![
            path_str.clone(),
            "test_topic".to_string(),
            "0.5".to_string(),
            "10".to_string(),
        ];

        run_with_args(args).expect("run should succeed");

        let content = read_to_string(&path).expect("file should exist");
        assert!(content.contains("\"topic\":\"test_topic\""));
        assert!(content.contains("\"value\":0.5"));
        assert!(content.contains("\"step\":10"));

        remove_file(path).ok();
    }

    #[test]
    fn test_missing_args() {
        let args = vec!["file".to_string(), "topic".to_string()];
        assert!(run_with_args(args).is_err());
    }

    #[test]
    fn test_invalid_types() {
        let args = vec![
            "file".to_string(),
            "topic".to_string(),
            "not_a_float".to_string(),
            "10".to_string(),
        ];
        assert!(run_with_args(args).is_err());
    }
}
