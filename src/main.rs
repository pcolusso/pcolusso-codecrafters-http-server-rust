use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;
use std::{net::TcpListener, io::Write};
use std::net::TcpStream;
use std::io::prelude::*;

use anyhow::{anyhow, Result};
use clap::Parser;
use once_cell::sync::OnceCell;
use strum_macros::{EnumString, Display};

#[derive(Debug, EnumString, Display)]
enum Verb {
    #[strum(serialize = "GET")]
    Get,
    #[strum(serialize = "POST")]
    Post
}

struct StartLine {
    verb: Verb,
    path: String
}

impl TryFrom<&str> for StartLine {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let stripped = value.strip_suffix("HTTP/1.1\r\n");

        if stripped.is_none() {
            return Err(anyhow!("Start line does not end with HTTP/1.1, it was '{:?}'", value));
        }

        let value = stripped.unwrap().trim().to_string();
        let components: Vec<&str>  = value.splitn(2, ' ').collect();

        if components.len() != 2 {
            return Err(anyhow!("Start line missing method or path"));
        }

        let verb = Verb::try_from(components[0])?;
        let path = components[1].to_string();
        
        Ok(StartLine { verb, path })
    }
}

#[derive(Debug)]
struct Header {
    key: String,
    value: String
}

impl TryFrom<&str> for Header {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let components: Vec<&str>  = value.splitn(2, ':').collect();

        if components.len() != 2 {
            return Err(anyhow::anyhow!("Header missing key or value"));
        }

        let key = components[0].trim().to_string();
        let value = components[1].trim().to_string();
        
        Ok(Header { key, value })
    }
}
struct Headers(Vec<Header>);

impl Headers {
    fn get(&self, key: &str) -> Option<String> {
        self.0.iter().find(|h| h.key == key).map(|h| h.value.clone())
    }

    fn add(&mut self, h: Header) {
        self.0.push(h);
    }
}

impl Header {
    fn is_header(line: &str) -> bool {
        let components: Vec<&str>  = line.splitn(2, ':').collect();

        components.len() == 2 
    }
}

struct Body(Vec<u8>);

struct Request(StartLine, Headers, Option<Body>);

fn read_stream(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream);
    let mut buf = String::with_capacity(50); 

    // Read the first line in
    reader.read_line(&mut buf)?;
    let start_line = StartLine::try_from(buf.as_str())?;
    buf.clear();

    // Headers, loop until we don't get anymore.
    let mut headers = Headers(vec!());
    loop {
        let _ = reader.read_line(&mut buf)?;
        eprintln!("Reading Headers, line is {:?}", buf);

        if Header::is_header(&buf) {
            headers.add(Header::try_from(buf.as_str())?);
        } else {
            break;
        }
        buf.clear();
    }

    // If we have Content-Length, there's a body to load.
    let body = match headers.get("Content-Length") {
        Some(content_length) => {
            let size: usize = content_length.parse()?;
            let mut body = vec![0; size];
            reader.read_exact(&mut body)?;
            Some(Body(body))
        },
        None => None
    };

    Ok(Request(start_line, headers, body))
}

fn make_text_response(status_code: u16, message: &str, body: &str) -> String {
    format!("HTTP/1.1 {status_code} {message}\r\nContent-Type: text/plain\r\nContent-Length: {0}\r\n\r\n{body}", body.len())
}

fn handle_request(stream: &mut TcpStream) -> Result<String> {
    let Request ( start_line, headers, body ) = read_stream(stream)?;
    let StartLine { verb, path } = start_line;
    let opts = Args::global();

    let response = match (verb, path.as_str(), body) {
        (Verb::Get, p, _) if p.starts_with("/echo/") => {
            let to_echo = p.strip_prefix("/echo/").unwrap(); // We just tested above
            make_text_response(200, "OK", to_echo)
        },
        (Verb::Post, p, Some(b)) if p.starts_with("/files/") => {
            let file_name = path.strip_prefix("/files/").unwrap();
            let file_path = opts.directory.clone().unwrap().join(file_name);
            let mut file = File::create(file_path)?;
            file.write_all(&b.0)?;
            make_text_response(201, "Created", "")
        },
        (Verb::Get, p, _) if opts.directory.is_some() && p.starts_with("/files/")  => {
            let file_name = path.strip_prefix("/files/").unwrap();
            let file_path = opts.directory.clone().unwrap().join(file_name);
            match std::fs::metadata(&file_path) {
                Ok(_) => {
                    let contents = std::fs::read_to_string(file_path)?;
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {0}\r\n\r\n{1}", contents.len(), contents)
                },
                Err(_) => make_text_response(404, "Not Found", "")
            }
        },
        (Verb::Get, "/user-agent", _) => {
            let user_agent = headers.get("User-Agent").unwrap(); // TODO: Handle
            make_text_response(200, "OK", &user_agent)
        },
        (Verb::Get, "/", _) => make_text_response(200, "OK", "body"),
        _ => make_text_response(404, "Not Found", "")
    };

    Ok(response)
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    directory: Option<PathBuf>
}

static OPTS: OnceCell<Args> = OnceCell::new();

impl Args {
    pub fn global() -> &'static Args {
        OPTS.get().expect("logger is not initialized")
    }
}


fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    {
        let args = Args::parse();
        OPTS.set(args).unwrap();
    }

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    match handle_request(&mut stream) {
                        Ok(response) => { 
                            stream.write_all(response.as_bytes()).unwrap();
                        },
                        Err(e) => { 
                            stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n500 Internal Server Error").unwrap();
                            eprintln!("Issue processing connection, {0}", e);
                        } 
                    }
                    stream.flush().unwrap();
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
