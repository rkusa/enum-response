extern crate http;

pub use http::StatusCode;

pub trait EnumResponse {
    fn status(&self) -> StatusCode;
    fn reason(&self) -> Option<&str> {
        self.status().canonical_reason()
    }
}
