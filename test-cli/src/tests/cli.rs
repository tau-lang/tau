use crate::{assert_none, assert_ok};
use tau_cli::{cli::ArgsBuilder, compile_file};
use tau_core::context::Context;

#[test]
fn pipeline_arrays() {
    let args = ArgsBuilder::new().build();
    let mut context = Context::new();
    let result = compile_file(&mut context, "../examples/arrays.tau", &args);
    assert_ok!(result);
    assert_none!(context.severity());
}

#[test]
fn pipeline_heap() {
    let args = ArgsBuilder::new().build();
    let mut context = Context::new();
    let result = compile_file(&mut context, "../examples/heap.tau", &args);
    assert_ok!(result);
    assert_none!(context.severity());
}

#[test]
fn pipeline_vec2() {
    let args = ArgsBuilder::new().build();
    let mut context = Context::new();
    let result = compile_file(&mut context, "../examples/vec2.tau", &args);
    assert_ok!(result);
    assert_none!(context.severity());
}
