#[macro_use]
extern crate api_error_derive;

#[derive(ErrorStatus)]
enum Error {
    #[response(reason_field = 1)]
    Tuple(String)
}

