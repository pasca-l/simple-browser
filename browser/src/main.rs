extern crate alloc;

use crate::alloc::string::ToString;
use net_std::http::HttpClient;

fn main() {
    let client = HttpClient::new();
    match client.get("example.com".to_string(), 80, "/".to_string()) {
        Ok(res) => {
            println!("response:\n{:#?}", res);
        }
        Err(e) => {
            println!("error:\n{:#?}", e);
        }
    }
}
