use crate::{assert_none, assert_ok};
use tau_cli::{compile_file, options::Options};
use tau_core::context::Context;

#[test]
fn pipeline_arrays() {
    let options = Options {
        input: "../examples/arrays.tau".into(),
        ..Default::default()
    };
    let mut context = Context::new();
    let result = compile_file(&mut context, &options);
    assert_ok!(result).unwrap();
    assert_none!(context.severity());
}

#[test]
fn pipeline_heap() {
    let options = Options {
        input: "../examples/heap.tau".into(),
        ..Default::default()
    };
    let mut context = Context::new();
    let result = compile_file(&mut context, &options);
    assert_ok!(result).unwrap();
    assert_none!(context.severity());
}

#[test]
fn pipeline_vec2() {
    let options = Options {
        input: "../examples/vec2.tau".into(),
        ..Default::default()
    };
    let mut context = Context::new();
    let result = compile_file(&mut context, &options);
    assert_ok!(result).unwrap();
    assert_none!(context.severity());
}
