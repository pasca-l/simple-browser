extern crate alloc;

// use crate::alloc::string::ToString;
use browser_core::browser::Browser;
// use browser_core::http::HttpResponse;
use alloc::rc::Rc;
use alloc::string::String;
use browser_core::error::Error;
use browser_core::http::HttpResponse;
use browser_core::url::Url;
use core::cell::RefCell;
use net_std::http::HttpClient;
use ui_cui::app::Tui;

// static TEST_HTTP_RESPONSE: &str = r#"HTTP/1.1 200 OK
// Data: xx xx xx

// <html>
// <head></head>
// <body>
//     <h1 id="title">H1 title</h1>
//     <h2 class="class">H2 title</h2>
//     <p>Test text.</p>
//     <p>
//         <a href="example.com">Link1</a>
//         <a href="example.com">Link2</a>
//     </p>
// </body>
// </html>
// "#;

fn handle_url(url: String) -> Result<HttpResponse, Error> {
    // parse url
    let parsed_url = match Url::new(url.to_string()).parse() {
        Ok(url) => url,
        Err(e) => {
            return Err(Error::UnexpectedInput(format!(
                "input html is not supported: {:?}",
                e
            )));
        }
    };

    // send a HTTP request and get a response
    let client = HttpClient::new();
    let response = match client.get(
        parsed_url.host(),
        parsed_url
            .port()
            .parse::<u16>()
            .unwrap_or_else(|_| panic!("port number should be u16 but got {}", parsed_url.port())),
        parsed_url.path(),
    ) {
        Ok(res) => {
            // redirect to Location
            if res.status_code() == 302 {
                let location = match res.header_value("Location") {
                    Ok(value) => value,
                    Err(_) => return Ok(res),
                };
                let redirect_parsed_url = Url::new(location);

                let redirect_client = HttpClient::new();
                match redirect_client.get(
                    redirect_parsed_url.host(),
                    redirect_parsed_url
                        .port()
                        .parse::<u16>()
                        .unwrap_or_else(|_| {
                            panic!("port number should be u16 but got {}", parsed_url.port())
                        }),
                    redirect_parsed_url.path(),
                ) {
                    Ok(res) => res,
                    Err(e) => return Err(Error::Network(format!("{:?}", e))),
                }
            } else {
                res
            }
        }
        Err(e) => {
            return Err(Error::Network(format!(
                "failed to get http response: {:?}",
                e
            )))
        }
    };

    Ok(response)
}

fn main() {
    let browser = Browser::new();

    let ui = Rc::new(RefCell::new(Tui::new(browser)));

    match ui.borrow_mut().start(handle_url) {
        Ok(_) => {}
        Err(e) => {
            println!("browser fails to start {:?}", e);
        }
    };

    // let response =
    //     HttpResponse::new(TEST_HTTP_RESPONSE.to_string()).expect("failed to parse http response");
    // let page = browser.borrow().current_page();
    // page.borrow_mut().receive_response(response);

    // let dom_string = page.borrow_mut().receive_response(response);
    // for log in dom_string.lines() {
    //     println!("{}", log);
    // }
}
