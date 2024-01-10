// Uncomment this block to pass the first stage
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread, vec,
};

use nom::AsBytes;
enum Method {
    GET,
    POST,
    Unknown,
}

struct Header<'a> {
    name: &'a str,
    content: &'a str,
}
struct HTTPRequest<'a> {
    method: Method,
    headers: Vec<Header<'a>>,
    path: &'a str,
    version: &'a str,
    content: &'a str,
}

impl<'a> HTTPRequest<'a> {
    pub fn new(data: Vec<&'a str>) -> Self {
        let head = data[0];
        let head: Vec<&str> = head.split(" ").collect();
        let mut headers: Vec<Header> = vec![];

        let data_iterrator = data.into_iter().skip(1);
        let mut is_body = false;
        let mut body: &str = "";

        for data in data_iterrator.clone() {
            if data.is_empty() {
                is_body = true
            }

            if is_body {
                body = data
            } else {
                let splited: Vec<&str> = data.split(": ").collect();

                headers.push(Header {
                    name: splited[0],
                    content: splited[1],
                })
            }
        }

        let method = if head[0] == "GET" {
            Method::GET
        } else if head[0] == "POST" {
            Method::POST
        } else {
            Method::Unknown
        };

        HTTPRequest {
            method,
            headers,
            path: if head.len() > 1 { head[1] } else { "" },
            version: if head.len() > 2 { head[2] } else { "" },
            content: body,
        }
    }

    //"HTTP/1.1 200 OK\r\n\r\n
}

enum StatusCode {
    Ok,
    NotFound,
    Created,
}

const EOL: &str = "\r\n";

struct HTTPResponse<'a> {
    stream: &'a mut TcpStream,
}

impl<'a> HTTPResponse<'a> {
    fn new(stream: &'a mut TcpStream) -> Self {
        HTTPResponse { stream }
    }

    fn base_send(self, version: &str, code: StatusCode, raw_content: &str, content_type: &str) {
        let head = format!(
            "{version} {}{EOL}",
            match code {
                StatusCode::Ok => "200 Ok",
                StatusCode::NotFound => "404 Not Found",
                StatusCode::Created => "201 Created",
            }
        );
        let headers = format!(
            "Content-Type: {content_type}{EOL}Content-Length: {}{EOL}",
            raw_content.len()
        );
        let content = format!("{raw_content}");
        let string = format!("{head}{headers}{EOL}{content}");

        self.stream.write(string.as_bytes()).unwrap();
    }

    fn send(self, version: &str, code: StatusCode, raw_content: &str) {
        self.base_send(version, code, raw_content, "text/plain")
    }

    fn send_file_content(self, version: &str, code: StatusCode, file_name: &str) {
        let content = fs::read_to_string(file_name).unwrap();
        self.base_send(version, code, content.as_str(), "application/octet-stream")
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0u8; 4096];
    match stream.read(&mut buffer) {
        Ok(_) => {
            let data = String::from_utf8_lossy(buffer.as_bytes());
            let data = data.trim_matches(char::from(0));
            let splited: Vec<&str> = data.lines().collect();

            let request = HTTPRequest::new(splited);

            let response = HTTPResponse::new(&mut stream);
            if request.path.starts_with("/echo/") {
                response.send(
                    request.version,
                    StatusCode::Ok,
                    request.path.strip_prefix("/echo/").unwrap(),
                );
            } else if request.path.starts_with("/files/") {
                //env args
                let args: Vec<String> = env::args().collect();
                let mut base_path = String::new();
                let mut next = false;

                for arg in args {
                    if arg == "--directory" {
                        next = true
                    }

                    if next {
                        base_path = arg;
                    }
                }

                let file_path = format!(
                    "{base_path}/{}",
                    request.path.strip_prefix("/files/").unwrap()
                );

                match request.method {
                    Method::GET => {
                        match fs::metadata(file_path.clone()) {
                            Ok(_) => {
                                //file exist
                                response.send_file_content(
                                    request.version,
                                    StatusCode::Ok,
                                    file_path.as_str(),
                                );
                            }
                            Err(_) => {
                                response.send(request.version, StatusCode::NotFound, "");
                            }
                        }
                    }
                    Method::POST => {
                        println!("Content: {}", request.content);

                        let mut file = File::create(file_path).unwrap();

                        println!("{}", request.content.len());

                        file.write(request.content.as_bytes()).unwrap();

                        response.send(request.version, StatusCode::Created, "");
                    }
                    Method::Unknown => todo!(),
                }
            } else if request.path == "/" {
                response.send(request.version, StatusCode::Ok, "");
            } else {
                for header in request.headers {
                    if header
                        .name
                        .eq_ignore_ascii_case(match request.path.strip_prefix("/") {
                            Some(text) => text,
                            None => "",
                        })
                    {
                        response.send(request.version, StatusCode::Ok, header.content);
                        return;
                    }
                }

                response.send(request.version, StatusCode::NotFound, "");
            }
        }
        Err(_) => println!("Unable to read data"),
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    //Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                thread::spawn(|| handle_connection(_stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
