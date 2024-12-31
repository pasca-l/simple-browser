use alloc::string::String;
use browser_core::error::Error;
use browser_core::http::HttpResponse;
use browser_core::url::Url;
use net_std::http::HttpClient;

pub fn handle_url(url: String) -> Result<HttpResponse, Error> {
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
