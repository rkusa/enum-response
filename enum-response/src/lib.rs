extern crate hyper;

pub use hyper::StatusCode;

pub trait EnumResponse {
    fn status(&self) -> StatusCode;
    fn reason(&self) -> Option<&str> {
        self.status().canonical_reason()
    }
}

// impl<T> Into<Response> for T where T: EnumResponse {
//     fn into(self) -> Response {
//         let mut res = Response::default().with_status(self.status());
//         if let Some(reason) = self.reason() {
//             res.set_body(reason);
//         }
//         res
//     }
// }

// impl<T> From<T> for Response where T: EnumResponse {
//     fn from(e: T) -> Self {
//         let mut res = Response::default().with_status(e.status());
//         if let Some(reason) = e.reason() {
//             res.set_body(reason);
//         }
//         res
//     }
// }