#[macro_use]
extern crate enum_response_derive;

#[derive(EnumResponse)]
enum Error {
    #[response(reason_field = 0)]
    Struct { s: String }
}

