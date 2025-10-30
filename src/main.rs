use morphogenetic_security::MorphogeneticApp;
use morphogenetic_security::cellular::SecurityCell;
use morphogenetic_security::telemetry::InMemorySink;

fn main() {
    let seed_cell = SecurityCell::new("root");
    let telemetry = InMemorySink::default();
    let mut app = MorphogeneticApp::new(vec![seed_cell], telemetry);

    app.step();

    let events = app.telemetry().events();
    println!(
        "Morphogenetic kernel scaffold executed one step; recorded {} telemetry event(s).",
        events.len()
    );
}
