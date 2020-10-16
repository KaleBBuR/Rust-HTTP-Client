/**
 * A Simple HTTP Client in Rust.
 * A message has a header part and a message body seperated by a blank line.
 * The blank line is ALWAYS needed even if there is no message body.
 * The header starts with a command and has additional lines of key value -
 * pairs seperated by a colon or a space.
 * If there is a message body, it  can be anything you want it to be.
 *
 * There are 2 main ways of submitting a request to a website
 *
 * * 1) GET:
 * * ---------------------------------------------------------------------------
 * * The query string is optional but, if specified, must be reasonably short
 * * Because of this the header could just be the GET command and nothing else.
 * * A sample message:
 * * GET /path?query_string HTTP/1.0\r\n
 * * \r\n
 *
 * * 2) POST:
 * * -------------------------------------------------------------------------------
 * * What would normally be in the query string is the body of the message instead.
 * * Because of this the header needs to include the
 * * Content-Type and Content-Length:
 * * attributes as well as the POST command.
 * * A sample message:
 * * POST /path HTTP/1.0\r\n
 * * Content-Type: text/plain\r\n
 * * Content-Length: 12\r\n
 * * \r\n
 * * query_string
 *
 * The send and receive calls won't necessarily send/receive ALL the data -
 * you give them - they will return the number of bytes actually sent/received.
 * It is up to you to call them in a loop and send/receive the remainder -7777
 * of the message.
 *
 *
 * If your request is big you can
 * * 1)
 * * Read the Content-Length: header from the response and then dynamically
 * * allocate enough memory to hold the whole response.
 *
 * * 2)
 * * Write the response to a file as the pieces arrive
 *
 * * What if you want to POST data in Wthe body of the message?
 * * Then you do need to include the Content-Type: and
 * * Content-Length: headers. The Content-Length: is the actual length
 * * of everything after the blank line that seperates the header from
 * * the body.
 *
 *
 * * Command Line Arguments
 * * 1) Host
 * * 2) Port
 * * 3) Command (GET or POST)
 * * 4) Path (not including the query data)
 * * 5) Query Data (put into the query string for GET and into the body for POST)
 * * 6) List of Headers (Content-Length: is automatic if using POST)
 *
 * CR = <US-ASCII CR, carriage return (13)>
 * LF = <US-ASCII LF, linefeed (10)>
 * HTTP/1.1 defines the sequence CR LF as the end-of-line marker for all protocol elements except the entity-body
 */

use std::collections::HashMap;
use std::net::TcpStream;
use std::fmt;
use std::io::prelude::*;
use std::io::BufReader;

use url::{Url, ParseError};
use native_tls::TlsConnector;
use regex::Regex;

fn main() { println!("goo goo gaa gaa"); }

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE
}

enum ResponseLine {
    FirstLine,
    Headers,
    Body
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HttpMethod::GET => formatter.write_str("GET"),
            HttpMethod::POST => formatter.write_str("POST"),
            HttpMethod::PUT => formatter.write_str("PUT"),
            HttpMethod::DELETE => formatter.write_str("DELETE")
        }
    }
}

trait Get {
    type Response;
    fn get(&mut self) -> Option<Self::Response>;
}

trait Post {
    type Response;
    fn post(&mut self) -> Option<Self::Response>;
}

trait Put {
    type Response;
    fn put(&mut self) -> Option<Self::Response>;
}

trait Delete {
    type Response;
    fn delete(&mut self) -> Option<Self::Response>;
}

#[derive(Debug)]
struct RequestConfig {
    url: url::Url,
    query: Option<HashMap<String, String>>,
    headers: Option<HashMap<String, String>>,
    user_agent: Option<String>,
    raw_data: Option<String>
}

#[derive(Debug)]
struct Request {
    config: RequestConfig,
    request: String,
    host: String,
    request_type: String
}

#[derive(Debug)]
struct Response {
    version: String,
    status_code: String,
    headers: HashMap<String, String>,
    body: String
}

impl RequestConfig {
    pub fn new<T, J, K, L, O, P>(
        url: &str,
        query: Option<HashMap<T, J>>,
        headers: Option<HashMap<K, L>>,
        user_agent: Option<O>,
        raw_data: Option<P>
    ) -> Result<Request, ParseError>
    where
        T: Into<String>,
        J: Into<String>,
        K: Into<String>,
        L: Into<String>,
        O: Into<String>,
        P: Into<String>
    {
        let config = Self {
            url: Url::parse(url)?,
            query: {
                match query {
                    Some(generic_query) => {
                        let mut string_query: HashMap<String, String> =  HashMap::new();
                        for (key, value) in generic_query.into_iter() {
                            string_query.insert(key.into(), value.into());
                        }
                        Some(string_query)
                    },
                    None => None
                }
            },
            headers: {
                match headers {
                    Some(generic_headers) => {
                        let mut string_headers: HashMap<String, String> =  HashMap::new();
                        for (key, value) in generic_headers.into_iter() {
                            string_headers.insert(key.into(), value.into());
                        }
                        Some(string_headers)
                    },
                    None => None
                }
            },
            user_agent: {
                match user_agent {
                    Some(generic_user_agent) => {
                        Some(generic_user_agent.into())
                    },
                    None => None
                }
            },
            raw_data: {
                match raw_data {
                    Some(generic_data) => {
                        Some(generic_data.into())
                    },
                    None => None
                }
            }
        };

        Ok(Request {
            config,
            request: String::new(),
            host: String::new(),
            request_type: String::new()
        })
    }
}

impl Request {
    fn setup_request(&mut self, method: HttpMethod) {
        let query = self.config.url.query();
        let mut path_query = self.config.url.path().to_string();
        let mut alr_query = false;
        let mut header_string = String::new();

        match query.is_some() {
            true => {
                alr_query = true;
                path_query.push('?');
                path_query.push_str(query.unwrap());
            },
            false => {}
        };

        match self.config.query.clone() {
            Some(query_map) => {
                match alr_query {
                    true => {},
                    false => path_query.push('?')
                };

                for (key, value) in query_map.iter() {
                    path_query.push_str(format!("{}={}&", key, value).as_str())
                }

                match path_query.strip_suffix("&") {
                    Some(stipped_query) => {
                        path_query = stipped_query.to_string();
                    },
                    None => {}
                };
            },
            None => {}
        };

        match self.config.url.host_str() {
            Some(host) => {
                self.host = host.to_string();
            },
            None => return
        };


        match self.config.headers.clone() {
            Some(headers) => {
                let mut found_connec = false;
                for (key, value) in headers.iter() {
                    if *key == String::from("Connection") { found_connec = true; }
                    header_string.push_str(format!("{}: {}\r\n", key, value).as_str());
                }

                if !found_connec {
                    header_string.push_str("Connection: keep-closed\r\n");
                }
            },
            None => {
                header_string.push_str("Connection: keep-closed\r\n");
            }
        };

        self.request = format!(
            "{} {} HTTP/1.1\r\n{}\r\n{}\r\n",
            method,
            path_query,
            format!("Host: {}", self.host),
            header_string
        );
    }

    fn send_request(&self) -> Option<Response> {
        let port: u16 = if self.config.url.scheme() == "https" { 443 } else { 80 };
        let address = format!("{}:{}", self.host, port);
        match port {
            443 => {
                let connector = TlsConnector::new().unwrap();
                let stream = TcpStream::connect(address).expect("Could not connect (Standard TCP)");
                let mut stream = connector.connect(&self.host.to_string(), stream).expect("Could not connect (TLS)");
                stream.write(self.request.as_bytes()).expect("Could not write to stream.");
                return self.read_response(stream);
            },
            80 => {
                let mut stream = TcpStream::connect(address).expect("Couldn't connect to address");
                stream.write(self.request.as_bytes()).expect("Unable to send request.");
                return self.read_response(stream);
            },
            _ => None
        }
    }

    fn read_response<T>(&self, stream: T) -> Option<Response>
    where
        T: std::io::Read
    {
        let reader = BufReader::new(stream);
        assert!(reader.buffer().len() == 0, "No request from reader!");
        let mut parsing = ResponseLine::FirstLine;

        let mut version = String::new();
        let mut status_code = String::new();
        let mut body = String::new();
        let mut headers: HashMap<String, String> = HashMap::new();

        for line in reader.lines() {
            let usable_line = line.expect("Can't read line.");
            match parsing {
                ResponseLine::FirstLine => {
                    let first_line_re = Regex::new(r"(?P<version>HTTP/[1-2].\d)\s(?P<code>[0-9]{3})").unwrap();
                    let caps = first_line_re.captures(usable_line.as_str()).expect("Couldn't regex string");
                    version = String::from(caps.name("version").expect("Missing HTTP Status Code").as_str());
                    status_code = String::from(caps.name("code").expect("Missing Status Code").as_str());
                    parsing = ResponseLine::Headers;
                },
                ResponseLine::Headers => {
                    if usable_line.is_empty() {
                        parsing = ResponseLine::Body;
                        continue;
                    }

                    let headers_re = Regex::new(r"(?P<key>[^:]+):\s(?P<value>[^:]+)").unwrap();
                    let caps = headers_re.captures(usable_line.as_str()).expect("Couldn't regex string");
                    let key = String::from(caps.name("key").expect("Can't get header key.").as_str());
                    let value = String::from(caps.name("value").expect("Can't get header value").as_str());
                    headers.insert(key, value);
                },
                ResponseLine::Body => {
                    body = format!("{}\n{}", body, usable_line);
                }
            }
        }

        body = String::from(body.strip_prefix("\n").unwrap());

        if status_code.eq("301") {
            let location = headers.get("Location").expect("Could not get Location header.");
            let mut request = RequestConfig::new::<String,String,String,String,String,String>(
                location,
                self.config.query.clone(),
                self.config.headers.clone(),
                self.config.user_agent.clone(),
                None
            ).unwrap();
            match self.request_type.as_str() {
                "GET" => return request.get(),
                "POST" => return request.post(),
                "PUT" => return request.put(),
                "DELETE" => return request.delete(),
                _ => {}
            }
        }

        Some(Response {
            version,
            status_code,
            headers,
            body
        })
    }
}

impl Get for Request {
    type Response = Response;

    fn get(&mut self) -> Option<Response> {
        self.request_type = String::from("GET");
        self.setup_request(HttpMethod::GET);
        self.send_request()
    }
}

impl Post for Request {
    type Response = Response;

    fn post(&mut self) -> Option<Response> {
        self.request_type = String::from("POST");
        self.setup_request(HttpMethod::POST);
        self.send_request()
    }
}

impl Put for Request {
    type Response = Response;

    fn put(&mut self) -> Option<Response> {
        self.request_type = String::from("PUT");
        self.setup_request(HttpMethod::PUT);
        self.send_request()
    }
}

impl Delete for Request {
    type Response = Response;

    fn delete(&mut self) -> Option<Response> {
        self.request_type = String::from("DELETE");
        self.setup_request(HttpMethod::DELETE);
        self.send_request()
    }
}

// <-- TESTS -->

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    struct OAuth {
        access_token: String,
        expires_in: usize,
        token_type: String
    }

    #[test]
    fn basic_request() {
        let mut client = RequestConfig::new::<&str, &str, &str, &str, &str, &str> (
            "https://www.google.com",
            None,
            None,
            None,
            None
        ).expect("Could not create new client.");
        println!("{:?}", client);
        // client.get();
    }

    #[test]
    fn query_check() {
        let mut client_query_check_1 = RequestConfig::new::<&str, &str, &str, &str, &str, &str> (
            "https://github.com/connorskees/requests/blob/master/src/request.rs",
            None,
            None,
            None,
            None
        ).expect("Could not create new client.");
        client_query_check_1.get();

        let mut query_map: HashMap<&str, &str> = HashMap::new();
        query_map.insert("p", "1868080");
        let mut client_query_check_2 = RequestConfig::new::<&str, &str, &str, &str, &str, &str> (
            "https://www.pearsonitcertification.com/articles/article.aspx",
            Some(query_map),
            None,
            None,
            None
        ).expect("Could not create new client.");
        client_query_check_2.get();
    }

    #[test]
    fn header_check() {
        let auth_url = "https://id.twitch.tv/oauth2/token";
        let mut auth_query_map: HashMap<&str, &str> = HashMap::new();
        auth_query_map.insert("client_id", "pyyp94iz0diuih4qzncipdzsd6ovj4");
        auth_query_map.insert("client_secret", "2m98yduy852d6iebpietc96kckdj4d");
        auth_query_map.insert("grant_type", "client_credentials");

        let response = RequestConfig::new::<&str, &str, &str, &str, &str, &str> (
            auth_url,
            Some(auth_query_map),
            None,
            None,
            None
        ).expect("Could not connect").post().expect("Couldn't get response.");

        eprintln!("{:?}", response);

        let oauth_json: OAuth = serde_json::from_str(&response.body).expect("Can't parse JSON");

        let url = "https://api.twitch.tv/helix/streams";
        let mut headers_map: HashMap<&str, &str> = HashMap::new();
        let token = format!("Bearer {}", oauth_json.access_token);
        headers_map.insert("Authorization", token.as_str());
        headers_map.insert("Client-Id", "pyyp94iz0diuih4qzncipdzsd6ovj4");
        let get_streams = RequestConfig::new::<&str, &str, &str, &str, &str, &str> (
            url,
            None,
            Some(headers_map),
            None,
            None
        ).expect("Could not connect").get().expect("Couldn't get response.");

        eprintln!("{:?}", get_streams.body);
    }
}