use std::fmt::Display;
use std::io::{Read, BufReader};
// Uncomment this block to pass the first stage
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

fn handle_request(mut stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut start_line = String::with_capacity(20); 
    reader.read_line(&mut start_line)?;

    let start_line = StartLine::try_from(start_line.as_str())?;

    let StartLine { path, verb } = start_line;

    eprintln!("Handing {verb} to {:?}", path);

    let response = match (verb, path.as_str()) {
        (Verb::Get, "/") => "HTTP/1.1 200 OK\r\n\r\n200 OK",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n404 Not Found"
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
                println!("accepted new connection");
                
                match handle_request(stream) {
                    Ok(()) => { },
                    Err(e) => { eprintln!("Issue processing connection, {0}", e); } 
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
