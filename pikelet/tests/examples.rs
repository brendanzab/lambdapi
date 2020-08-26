//! Integration tests against the language samples directory.

use pikelet::{core, surface};

fn run_test(input: &str) {
    let mut is_failed = false;

    let surface_term = surface::Term::from_str(input).unwrap();

    let globals = core::Globals::default();
    let mut state = surface::projections::core::State::new(&globals);
    let (core_term, r#type) = surface::projections::core::synth_term(&mut state, &surface_term);
    let errors = state.drain_errors().collect::<Vec<_>>();
    if !errors.is_empty() {
        is_failed = true;
        eprintln!("surface::projections::core::synth_term errors:");
        for error in errors {
            eprintln!("  {:?}", error);
        }
        eprintln!();
    }

    let mut state = core::typing::State::new(&globals);
    core::typing::synth_term(&mut state, &core_term);
    let errors = state.drain_errors().collect::<Vec<_>>();
    if !errors.is_empty() {
        is_failed = true;
        eprintln!("core::typing::synth_term errors:");
        for error in errors {
            eprintln!("  {:?}", error);
        }
        eprintln!();
    }

    let mut state = core::typing::State::new(&globals);
    core::typing::check_term(&mut state, &core_term, &r#type);
    let errors = state.drain_errors().collect::<Vec<_>>();
    if !errors.is_empty() {
        is_failed = true;
        eprintln!("core::typing::check_term errors:");
        for error in errors {
            eprintln!("  {:?}", error);
        }
        eprintln!();
    }

    if is_failed {
        panic!("failed sample");
    }
}

#[test]
fn cube() {
    run_test(include_str!("../../examples/cube.pi"));
}

#[test]
fn functions() {
    run_test(include_str!("../../examples/functions.pi"));
}

#[test]
fn hello_world() {
    run_test(include_str!("../../examples/hello-world.pi"));
}

#[test]
fn module() {
    run_test(include_str!("../../examples/module.pi"));
}

#[test]
fn universes() {
    run_test(include_str!("../../examples/universes.pi"));
}

#[test]
fn window_settings() {
    run_test(include_str!("../../examples/window-settings.pi"));
}
