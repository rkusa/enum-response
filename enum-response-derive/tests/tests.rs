#![feature(attr_literals)]

#[macro_use]
extern crate enum_response_derive;
extern crate enum_response;
extern crate compiletest_rs as compiletest;

use std::path::PathBuf;
use enum_response::{EnumResponse, StatusCode};

fn run_mode(mode: &'static str) {
    let mut config = compiletest::Config::default();

    config.mode = mode.parse().expect("Invalid mode");
    config.src_base = PathBuf::from(format!("tests/{}", mode));
    config.link_deps(); // Populate config.target_rustcflags with dependencies on the path

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("ui");
}

#[test]
fn default_internal_server_error() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        Unit,
        Tuple(&'a str),
        Struct { s: &'a str },
    }
    assert_eq!(Error::Unit.status(), StatusCode::InternalServerError);
    assert_eq!(Error::Unit.reason(), Some("Internal Server Error"));
    assert_eq!(Error::Tuple("").status(), StatusCode::InternalServerError);
    assert_eq!(Error::Tuple("").reason(), Some("Internal Server Error"));
    assert_eq!(
        Error::Struct { s: "" }.status(),
        StatusCode::InternalServerError
    );
    assert_eq!(
        Error::Struct { s: "" }.reason(),
        Some("Internal Server Error")
    );
}

#[test]
fn override_status_code_int() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(status = 400)]
        Unit,
        #[response(status = 401)]
        Tuple(&'a str),
        #[response(status = 402)]
        Struct { s: &'a str },
    }
    assert_eq!(Error::Unit.status(), StatusCode::BadRequest);
    assert_eq!(Error::Tuple("").status(), StatusCode::Unauthorized);
    assert_eq!(
        Error::Struct { s: "" }.status(),
        StatusCode::PaymentRequired
    );
}

#[test]
fn override_status_code_string() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(status = "402")]
        Unit,
        #[response(status = "400")]
        Tuple(&'a str),
        #[response(status = "401")]
        Struct { s: &'a str },
    }
    assert_eq!(Error::Unit.status(), StatusCode::PaymentRequired);
    assert_eq!(Error::Tuple("").status(), StatusCode::BadRequest);
    assert_eq!(Error::Struct { s: "" }.status(), StatusCode::Unauthorized);
}

#[test]
fn override_status_name() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(status = "Unauthorized")]
        Unit,
        #[response(status = "PaymentRequired")]
        Tuple(&'a str),
        #[response(status = "BadRequest")]
        Struct { s: &'a str },
    }
    assert_eq!(Error::Unit.status(), StatusCode::Unauthorized);
    assert_eq!(Error::Tuple("").status(), StatusCode::PaymentRequired);
    assert_eq!(Error::Struct { s: "" }.status(), StatusCode::BadRequest);
}

#[test]
fn override_reason() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(reason = "A")]
        Unit,
        #[response(reason = "B")]
        Tuple(&'a str),
        #[response(reason = "C")]
        Struct { s: &'a str },
    }
    assert_eq!(Error::Unit.reason(), Some("A"));
    assert_eq!(Error::Tuple("").reason(), Some("B"));
    assert_eq!(Error::Struct { s: "" }.reason(), Some("C"));
}

#[test]
fn override_reason_from_tuple_field() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(reason_field = 0)]
        One(String),
        #[response(reason_field = 1)]
        Two(&'a str, &'a str),
        #[response(reason_field = 0)]
        First(&'a str, &'a str, &'a str),
        #[response(reason_field = 1)]
        Inbetween(&'a str, &'a str, &'a str),
        #[response(reason_field = 2)]
        Last(&'a str, &'a str, &'a str),
    }

    assert_eq!(Error::One(String::from("a")).reason(), Some("a"));
    assert_eq!(Error::Two("a", "b").reason(), Some("b"));
    assert_eq!(Error::First("a", "b", "c").reason(), Some("a"));
    assert_eq!(Error::Inbetween("a", "b", "c").reason(), Some("b"));
    assert_eq!(Error::Last("a", "b", "c").reason(), Some("c"));
}

#[test]
fn struct_variant() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        Struct { s: &'a str },
    }
    let err = Error::Struct { s: "" };
    assert_eq!(err.status(), StatusCode::InternalServerError);
    assert_eq!(err.reason(), Some("Internal Server Error"));
}

#[test]
fn override_reason_from_struct_field() {
    #[derive(Debug, EnumResponse)]
    enum Error<'a> {
        #[response(reason_field = "a")]
        First { a: &'a str, b: &'a str, c: &'a str },
        #[response(reason_field = "b")]
        Second { a: &'a str, b: &'a str, c: &'a str },
        #[response(reason_field = "c")]
        Third { a: &'a str, b: &'a str, c: &'a str },
    }
    assert_eq!(
        Error::First {
            a: "1",
            b: "2",
            c: "3",
        }.reason(),
        Some("1")
    );
    assert_eq!(
        Error::Second {
            a: "1",
            b: "2",
            c: "3",
        }.reason(),
        Some("2")
    );
    assert_eq!(
        Error::Third {
            a: "1",
            b: "2",
            c: "3",
        }.reason(),
        Some("3")
    );
}