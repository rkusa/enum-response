#[macro_use]
extern crate enum_response_derive;

#[derive(EnumResponse)]
enum Error {
    #[response(reason_field = 1)]
    Tuple(String)
}

