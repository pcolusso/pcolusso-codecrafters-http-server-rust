use std::fmt::Display;
use std::io::BufReader;
use std::thread;
use std::{net::TcpListener, io::Write};
use std::net::TcpStream;
use std::io::prelude::*;

use anyhow::{anyhow, Result};

enum Verb {
    Get
}

impl Display for Verb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verb::Get => write!(f, "GET")?,
        }

        Ok(())
    }
}

impl TryFrom<&str> for Verb {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Verb::Get),
            _ => Err(anyhow!("Unknown verb {value}"))
        }
    }
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
            return Err(anyhow::anyhow!("Start line does not end with HTTP/1.1, it was '{:?}'", value));
        }

        let value = stripped.unwrap().trim().to_string();
        let components: Vec<&str>  = value.splitn(2, ' ').collect();

        if components.len() != 2 {
            return Err(anyhow::anyhow!("Start line missing method or path"));
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

impl Header {
    fn is_header(line: &str) -> bool {
        let components: Vec<&str>  = line.splitn(2, ':').collect();

        components.len() == 2 
    }
}

fn handle_request(mut stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut buf = String::with_capacity(50); 
    reader.read_line(&mut buf)?;

    let start_line = StartLine::try_from(buf.as_str())?;
    buf.clear();

    let StartLine { path, verb } = start_line;

    eprintln!("Handing {verb} to {:?}", path);

    let mut headers: Vec<Header> = vec!();

    loop {
        let _ = reader.read_line(&mut buf)?;
        eprintln!("Reading Headers, line is {:?}", buf);

        if Header::is_header(&buf) {
            headers.push(Header::try_from(buf.as_str())?);
        } else {
            break;
        }
        buf.clear();
    }

    eprintln!("Headers: {:?}", headers);

    let response = if path.starts_with("/echo/") {
        let to_echo = path.strip_prefix("/echo/").unwrap();
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {0}\r\n\r\n{to_echo}", to_echo.len())
    } else if path == "/user-agent" {
        let user_agent = headers.iter().find(|h| h.key == "User-Agent").unwrap().value.clone();
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {0}\r\n\r\n{1}", user_agent.len(), user_agent)
    } else if path == "/" {
        "HTTP/1.1 200 OK\r\n\r\n200 OK".to_string()
    } else {
        "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found".to_string()
    };

    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    
    Ok(())
}

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    match handle_request(stream) {
                        Ok(()) => { },
                        Err(e) => { eprintln!("Issue processing connection, {0}", e); } 
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
