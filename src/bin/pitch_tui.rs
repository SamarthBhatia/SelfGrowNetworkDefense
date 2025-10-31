use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use serde::Deserialize;
use std::cmp::Ordering;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/pitch_demo"));

    let mut app = App::load(dir)?;
    run_app(&mut app)?;
    Ok(())
}

struct App {
    dir: PathBuf,
    outcomes: Vec<OutcomeDisplay>,
    message: String,
}

impl App {
    fn load(dir: PathBuf) -> Result<Self, Box<dyn Error>> {
        let outcomes = load_outcomes(&dir)?;
        let message = if outcomes.is_empty() {
            format!(
                "No *_outcome.json files found in {}. Run scripts/pitch_demo.sh first.",
                dir.display()
            )
        } else {
            format!(
                "Loaded {} scenario(s) from {}. Press 'r' to reload, 'q' to quit.",
                outcomes.len(),
                dir.display()
            )
        };
        Ok(Self {
            dir,
            outcomes,
            message,
        })
    }

    fn reload(&mut self) {
        match load_outcomes(&self.dir) {
            Ok(outcomes) => {
                self.message = if outcomes.is_empty() {
                    format!(
                        "No *_outcome.json files found in {}. Run scripts/pitch_demo.sh first.",
                        self.dir.display()
                    )
                } else {
                    format!(
                        "Reloaded {} scenario(s) from {}.",
                        outcomes.len(),
                        self.dir.display()
                    )
                };
                self.outcomes = outcomes;
            }
            Err(err) => {
                self.message = format!("Reload failed: {err}");
            }
        }
    }
}

fn run_app(app: &mut App) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| {
            let size = frame.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(2),
                    ]
                    .as_ref(),
                )
                .split(size);

            let instructions = Paragraph::new(vec![
                Line::from(Span::styled(
                    "Morphogenetic Pitch Dashboard",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from("Press 'r' to reload outcomes, 'q' to quit."),
            ])
            .block(Block::default().borders(Borders::ALL).title("Controls"));
            frame.render_widget(instructions, chunks[0]);

            if app.outcomes.is_empty() {
                let empty = Paragraph::new(app.message.clone())
                    .block(Block::default().borders(Borders::ALL).title("Status"));
                frame.render_widget(empty, chunks[1]);
            } else {
                let table = build_table(&app.outcomes);
                frame.render_widget(table, chunks[1]);
            }

            let mut summary_lines = Vec::new();
            for outcome in &app.outcomes {
                let mutation = outcome
                    .recommended_mutation
                    .as_deref()
                    .unwrap_or("No mutation suggested");
                let next = outcome
                    .next_candidate
                    .as_deref()
                    .unwrap_or("No follow-up queued");
                summary_lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}: ", outcome.label),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("mutation → {mutation}; next → {next}")),
                ]));
            }
            if summary_lines.is_empty() {
                summary_lines.push(Line::from(app.message.clone()));
            }
            let summary = Paragraph::new(summary_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Mutations / Backlog"),
            );
            frame.render_widget(summary, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => app.reload(),
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}

fn build_table(outcomes: &[OutcomeDisplay]) -> Table<'static> {
    let mut header_cells = vec![Cell::from("Metric")];
    for outcome in outcomes {
        header_cells.push(Cell::from(format!(
            "{} (gen {})",
            outcome.label, outcome.generation
        )));
    }

    let mut rows = Vec::new();

    add_row(&mut rows, "Fitness score", outcomes, |o| {
        format!("{:.3}", o.fitness_score)
    });
    add_row(&mut rows, "Breach observed", outcomes, |o| {
        if o.breach_observed {
            "yes".to_string()
        } else {
            "no".to_string()
        }
    });
    add_row(&mut rows, "Simulation steps", outcomes, |o| {
        o.step_count.to_string()
    });
    add_row(&mut rows, "Avg threat", outcomes, |o| {
        format!("{:.2}", o.avg_threat)
    });
    add_row(&mut rows, "Max threat", outcomes, |o| {
        format!("{:.2}", o.max_threat)
    });
    add_row(&mut rows, "Total replications", outcomes, |o| {
        o.total_replications.to_string()
    });
    add_row(&mut rows, "Total signals", outcomes, |o| {
        o.total_signals.to_string()
    });
    add_row(&mut rows, "Total stimulus", outcomes, |o| {
        format!("{:.2}", o.total_stimulus)
    });
    add_row(&mut rows, "Cell count range", outcomes, |o| {
        format!("{} → {}", o.min_cell_count, o.max_cell_count)
    });

    let mut widths = vec![Constraint::Percentage(25)];
    if !outcomes.is_empty() {
        let per = (75 / outcomes.len() as u16).max(10);
        for _ in outcomes {
            widths.push(Constraint::Percentage(per));
        }
    }

    let table = Table::new(rows, widths)
        .header(Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Scenario Comparison"),
        )
        .column_spacing(2);
    table
}

fn add_row<F>(
    rows: &mut Vec<Row<'static>>,
    label: &str,
    outcomes: &[OutcomeDisplay],
    mut value_fn: F,
) where
    F: FnMut(&OutcomeDisplay) -> String,
{
    let mut cells = Vec::with_capacity(outcomes.len() + 1);
    cells.push(Cell::from(label.to_string()));
    for outcome in outcomes {
        cells.push(Cell::from(value_fn(outcome)));
    }
    rows.push(Row::new(cells));
}

fn load_outcomes(dir: &Path) -> Result<Vec<OutcomeDisplay>, Box<dyn Error>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with("_outcome.json"))
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by(|a, b| match (a.file_name(), b.file_name()) {
        (Some(a_name), Some(b_name)) => a_name.cmp(b_name),
        _ => Ordering::Equal,
    });

    let mut outcomes = Vec::new();
    for entry in entries {
        match parse_outcome(&entry) {
            Ok(outcome) => outcomes.push(outcome),
            Err(err) => eprintln!("Failed to parse {}: {err}", entry.display()),
        }
    }

    Ok(outcomes)
}

fn parse_outcome(path: &Path) -> Result<OutcomeDisplay, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let payload: OutcomeFile = serde_json::from_reader(reader)?;

    let label = payload
        .outcome
        .candidate_id
        .split('-')
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| payload.outcome.candidate_id.clone());

    let next_candidate = payload
        .next_candidate
        .map(|candidate| match candidate.mutation_note {
            Some(note) if !note.is_empty() => format!("{} – {}", candidate.id, note),
            _ => candidate.id,
        });

    Ok(OutcomeDisplay {
        label,
        generation: payload.outcome.generation,
        fitness_score: payload.outcome.fitness_score,
        breach_observed: payload.outcome.breach_observed,
        step_count: payload.statistics.step_count,
        avg_threat: payload.statistics.avg_threat,
        max_threat: payload.statistics.max_threat,
        total_replications: payload.statistics.total_replications,
        total_signals: payload.statistics.total_signals,
        total_stimulus: payload.statistics.total_stimulus,
        min_cell_count: payload.statistics.min_cell_count,
        max_cell_count: payload.statistics.max_cell_count,
        recommended_mutation: payload.recommended_mutation,
        next_candidate,
    })
}

#[derive(Debug, Deserialize)]
struct OutcomeFile {
    outcome: OutcomeSnapshot,
    statistics: OutcomeStats,
    #[serde(default)]
    recommended_mutation: Option<String>,
    #[serde(default)]
    next_candidate: Option<NextCandidate>,
}

#[derive(Debug, Deserialize)]
struct OutcomeSnapshot {
    candidate_id: String,
    generation: u32,
    fitness_score: f32,
    breach_observed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OutcomeStats {
    step_count: usize,
    avg_threat: f32,
    max_threat: f32,
    avg_cell_count: f32,
    min_cell_count: usize,
    max_cell_count: usize,
    total_replications: u32,
    total_signals: u32,
    total_lineage_shifts: u32,
    total_stimulus: f32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct NextCandidate {
    id: String,
    scenario_ref: String,
    generation: u32,
    #[serde(default)]
    mutation_note: Option<String>,
}

struct OutcomeDisplay {
    label: String,
    generation: u32,
    fitness_score: f32,
    breach_observed: bool,
    step_count: usize,
    avg_threat: f32,
    max_threat: f32,
    total_replications: u32,
    total_signals: u32,
    total_stimulus: f32,
    min_cell_count: usize,
    max_cell_count: usize,
    recommended_mutation: Option<String>,
    next_candidate: Option<String>,
}
