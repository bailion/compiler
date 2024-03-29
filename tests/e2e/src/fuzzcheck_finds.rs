//! Test-cases which were found using Fuzzcheck.

use logic::{
    diagnostics::reportable_error::ReportableError,
    parse::utils::ParseError,
    ty::error::{ConstraintGatheringError, TyCheckError},
};

fn compile_for_fuzzing(input: &str) {
    let table = logic::parse::parse(input).unwrap();
    if let Ok(ty_checked) = logic::ty::type_check(&table) {
        let _ = logic::codegen::codegen(&table, &ty_checked);
    }
}

/// Describes the status of a program we attempted to run.
enum ExecutionStatus {
    /// All stages ran successfully.
    ///
    /// The number is the exit code returned by the compiled program.
    Ok(i32),
    /// The program could not be parsed.
    FailedParsing(ParseError),
    /// The program could not be type checked.
    FailedTypeChecking(TyCheckError),
    /// The relevant machine code could not be generated for the program.
    FailedCodeGeneration(ReportableError),
}

impl ExecutionStatus {
    fn as_failed_code_generation(&self) -> Option<&ReportableError> {
        if let Self::FailedCodeGeneration(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_failed_parsing(&self) -> Option<&ParseError> {
        if let Self::FailedParsing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_failed_type_checking(&self) -> Option<&TyCheckError> {
        if let Self::FailedTypeChecking(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Runs the provided code fragment, returning the status of the execution.
fn run_test(input: &str) -> ExecutionStatus {
    let tree = match logic::parse::parse(input) {
        Ok(tree) => tree,
        Err(error) => return ExecutionStatus::FailedParsing(error),
    };

    let ty_env = match logic::ty::type_check(&tree) {
        Ok(env) => env,
        Err(err) => return ExecutionStatus::FailedTypeChecking(err),
    };

    let codegen = match logic::codegen::codegen(&tree, &ty_env) {
        Ok(res) => res,
        Err(err) => return ExecutionStatus::FailedCodeGeneration(err),
    };

    ExecutionStatus::Ok(codegen)
}

#[test]
fn empty_function_with_one_parameter() {
    let status = run_test("function P (t,)\nendfunction\nP = False\n");
    let error = status.as_failed_code_generation().unwrap();
    dbg!(&error);
    assert!(error
        .explanation()
        // todo: this message could be improved
        .contains("A type of variable could not be established for this function parameter."));
}

#[test]
fn function_and_record() {
    compile_for_fuzzing("function P ()\n  +s { k: False,}\nendfunction\nP = False\n");
}

#[test]
#[ignore = "todo: implement boolean literals"]
fn empty_function_with_boolean_literal() {
    compile_for_fuzzing("function P ()\n  False\nendfunction\nP = False\n");
}

#[test]
fn boolean_field_access_inside_function() {
    let result = run_test("function P ()\n  True.H\nendfunction");
    let error = result.as_failed_code_generation().unwrap();
    assert!(error.explanation().contains("return type"));
    assert!(error.explanation().contains("could not"));
}

#[test]
#[ignore = "todo: implement support for string literals in expression positions"]
fn string_literal() {
    compile_for_fuzzing("function P ()\n  N = \"i\"\nendfunction\nP = False\n");
}

#[test]
#[ignore = "todo: fix incorrect use of Cranelift"]
fn bool_inside_func() {
    compile_for_fuzzing("function P ()\n  record U\n  endrecord\nendfunction\nP = False");
}

#[test]
// TODO: check error message for this
fn constructor_without_record_definition() {
    compile_for_fuzzing("k { }");
}

#[test]
#[ignore = "todo: implement support for methods"]
fn boolean_method_call() {
    compile_for_fuzzing("True.m()\n");
}

#[test]
fn loop_and_record_inside_function() {
    compile_for_fuzzing("function Q ()\n  while i { }\n  endwhile\nendfunction\n");
}

#[test]
fn while_with_function_call_inside_function() {
    let binding = run_test("function Q ()\n  while R(p,)\n  endwhile\nendfunction\nQ = False\n");
    let error = binding.as_failed_code_generation().unwrap();
    assert!(error.explanation().contains("type"));
    assert!(error.explanation().contains("function parameter"));
}

#[test]
fn invalid_return() {
    let result = run_test("while return True\nendwhile");
    result.as_failed_parsing().unwrap();
}

#[test]
#[ignore = "todo: fix type checking for this function"]
fn bad_expressions() {
    let result = run_test("O()-4957547627119355237\nfunction O ()\n  O()-False\nendfunction\n");
    result.as_failed_type_checking().unwrap();

    let result =
        run_test("for v = False to True\nnext v\nfunction v ()\n  h = +-False\nendfunction\n");
    result.as_failed_type_checking().unwrap();
}

#[test]
fn empty_string() {
    let result = run_test("");
    let error = result.as_failed_code_generation().unwrap();
    assert!(error.explanation().contains("`main` function"))
}

#[test]
fn invalid_bool_access() {
    let result = run_test("function L ()\n  False.k\n  return   False\nendfunction\n");
    let error = result.as_failed_code_generation().unwrap();
    assert!(error.explanation().contains("used"));
    assert!(error.explanation().contains("never defined"));
}

#[test]
#[ignore = "todo: should not type check"]
fn bad_expression() {
    compile_for_fuzzing("function t ()\n  return   -False==-False\nendfunction\n");
}

#[test]
#[ignore = "todo: fix Cranelift IR generation"]
fn type_not_declared() {
    compile_for_fuzzing("g = True\nfunction g ()\n  T\nendfunction\n");
}

#[test]
fn function_with_while_and_boolean() {
    compile_for_fuzzing("g = False\nfunction g ()\n  while True==False\n  endwhile\nendfunction\n");
}

#[test]
#[ignore = "todo: fix Cranelift IR generation for this function"]
fn infinite_loop_of_functions() {
    compile_for_fuzzing("function v (v,)\n  return   v(True!=+v(),)\nendfunction\n");
}

#[test]
fn return_bool() {
    compile_for_fuzzing("function l ()\n  return   True\n  n = True\nendfunction\n");
}

#[test]
fn undefined_record() {
    let result = run_test("function main()\n  Record{}\nendfunction");
    let error = result.as_failed_type_checking().unwrap();
    let cge = error.as_constraint_gathering_error().unwrap();
    if let ConstraintGatheringError::UnresolvableRecord {
        span: _,
        explanation,
    } = cge
    {
        assert!(explanation.contains("record"));
    } else {
        dbg!(cge);
        panic!("wrong error type");
    }
}

#[cfg(test)]
#[test]
#[ignore = "todo: fix type checking/argument checking for this function"]
fn two_functions() {
    compile_for_fuzzing(
        "function h (k,)\n  return   False==h(True,)\nendfunction\nfunction h ()\nendfunction\n",
    );
}

#[cfg(test)]
#[test]
fn boolean_creation() {
    compile_for_fuzzing("function V ()\n  M = True\n  return   False\nendfunction\n");
}

#[test]
fn func_v_compiles() {
    compile_for_fuzzing(include_str!("../../../logic/src/ty/examples/V"));
}
