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
use serde::{Serialize, Deserialize};

fn main() { println!("goo goo gaa gaa"); }

#[derive(Serialize, Deserialize)]
struct OAuth {
    access_token: String,
    expires_in: usize,
    token_type: String
}

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

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let empty_hashmap: HashMap<String, String> = HashMap::new();
        write!(f, "URL: {}\nQUERY: {:?}\nHEADERS: {:?}\nUSER AGENT: {}\nREQUEST: {}",
            self.url,
            self.query.as_ref().unwrap_or(&empty_hashmap),
            self.headers.as_ref().unwrap_or(&empty_hashmap),
            self.user_agent.as_ref().unwrap_or(&"".to_string()),
            self.request
        )
    }
}

struct Request {
    url: url::Url,
    query: Option<HashMap<String, String>>,
    headers: Option<HashMap<String, String>>,
    user_agent: Option<String>,
    request: String,
    host: String,
    rt: String
}

#[derive(Debug)]
struct Response {
    version: String,
    status_code: String,
    headers: HashMap<String, String>,
    body: String
}

impl Request {
    pub fn new(url: &str, query: Option<HashMap<String, String>>, headers: Option<HashMap<String, String>>, user_agent: Option<String>) -> Result<Request, ParseError> {
        let parsed_url = Url::parse(url)?;
        Ok(Self {
            url: parsed_url,
            query,
            headers,
            user_agent,
            request: String::new(),
            host: String::new(),
            rt: String::new()
        })
    }

    fn setup_request(&mut self, method: HttpMethod) {
        let query = self.url.query();
        let mut path_query = self.url.path().to_string();
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

        match self.query.clone() {
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

        match self.url.host_str() {
            Some(host) => {
                self.host = host.to_string();
            },
            None => return
        };


        match self.headers.clone() {
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
        let port: u16 = if self.url.scheme() == "https" { 443 } else { 80 };
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
            let mut request = Request::new(location, self.query.clone(), self.headers.clone(), self.user_agent.clone()).unwrap();
            match self.rt.as_str() {
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

    pub fn get(&mut self) -> Option<Response> {
        self.rt = String::from("GET");
        self.setup_request(HttpMethod::GET);
        self.send_request()
    }

    pub fn post(&mut self) -> Option<Response> {
        self.rt = String::from("POST");
        self.setup_request(HttpMethod::POST);
        self.send_request()
    }

    pub fn put(&mut self) -> Option<Response> {
        self.rt = String::from("PUT");
        self.setup_request(HttpMethod::PUT);
        self.send_request()
    }

    pub fn delete(&mut self) -> Option<Response> {
        self.rt = String::from("DELETE");
        self.setup_request(HttpMethod::DELETE);
        self.send_request()
    }
}

// <-- TESTS -->

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_request() {
        let mut client = Request::new("https://www.google.com", None, None, None).expect("Could not create new client.");
        println!("{}", client);
        client.get();
    }

    #[test]
    fn query_check() {
        let mut client_query_check_1 = Request::new("https://github.com/connorskees/requests/blob/master/src/request.rs", None, None, None).expect("Could not create new client.");
        client_query_check_1.get();

        let mut query_map: HashMap<String, String> = HashMap::new();
        query_map.insert("p".to_string(), "1868080".to_string());
        let mut client_query_check_2 = Request::new("https://www.pearsonitcertification.com/articles/article.aspx", Some(query_map), None, None).expect("Could not create new client.");
        client_query_check_2.get();
    }

    #[test]
    fn header_check() {
        let auth_url = "https://id.twitch.tv/oauth2/token";
        let mut auth_query_map: HashMap<String, String> = HashMap::new();
        auth_query_map.insert("client_id".to_string(), "pyyp94iz0diuih4qzncipdzsd6ovj4".to_string());
        auth_query_map.insert("client_secret".to_string(), "2m98yduy852d6iebpietc96kckdj4d".to_string());
        auth_query_map.insert("grant_type".to_string(), "client_credentials".to_string());

        let response = Request::new(
            auth_url,
            Some(auth_query_map),
            None,
            None
        ).expect("Could not connect").post().expect("Couldn't get response.");

        eprintln!("{:?}", response);

        let oauth_json: OAuth = serde_json::from_str(&response.body).expect("Can't parse JSON");

        let url = "https://api.twitch.tv/helix/streams";
        let mut headers_map: HashMap<String, String> = HashMap::new();
        headers_map.insert("Authorization".to_string(), format!("Bearer {}", oauth_json.access_token));
        headers_map.insert("Client-Id".to_string(), "pyyp94iz0diuih4qzncipdzsd6ovj4".to_string());
        let get_streams = Request::new(
            url,
            None,
            Some(headers_map),
            None
        ).expect("Could not connect").get().expect("Couldn't get response.");

        eprintln!("{}", get_streams.body);
    }
}