use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, io::{Read, Write}
};

use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
struct Body {
    content: String,
    encoding: String,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsontpRequest {
    jsontp: String,
    #[serde(rename = "type")]
    type_of_request: String,
    method: String,
    resource: String,
    headers: HashMap<String, Value>,
    body: Body,
}

#[derive(Serialize, Deserialize, Debug)]
struct Status {
    code: u16,
    #[serde(rename = "formal-message")]
    formal_message: String,
    #[serde(rename = "human-message")]
    human_message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsontpResponse {
    jsontp: String,
    #[serde(rename = "type")]
    type_of_response: String,
    status: Status,
    resource: String,
    headers: HashMap<String, Value>,
    body: Body,
}

impl JsontpRequest {
    fn validate(&self) -> Result<(), (String, u16)> {
        if self.jsontp.get(..3) != Some("1.0") {
            return Err(("HTTP Version Not Supported".to_string(), 505));
        }
        if self.type_of_request != "request" {
            return Err(("Bad Request".to_string(), 400));
        }
        if self.resource.is_empty() {
            return Err(("Bad Request".to_string(), 400));
        }
        if self.body.content.is_empty() {
            return Err(("Bad Request".to_string(), 400));
        }
        if self.body.encoding.is_empty() {
            return Err(("Bad Request".to_string(), 400));
        }
        if self.method.is_empty() {
            return Err(("Bad Request".to_string(), 400));
        }

        match self.body.encoding.as_str() {
            "gzip" | "deflate" | "br" | "identity" => {}

            _ => return Err(("Bad Request".to_string(), 400)),
        }

        match self.method.as_str() {
            "GET" | "POST" | "PUT" | "DELETE" | "OPTIONS" => {}
            _ => return Err(("Bad Request".to_string(), 400)),
        }

        let mut bad_headers = false;

        for (key, _) in &self.headers {
            match key.to_lowercase().as_str() {
                "content-type"
                | "accept"
                | "accept-encoding"
                | "accept-language"
                | "authorization"
                | "cookies"
                | "if-modified-since"
                | "if-unmodified-since"
                | "expect" => {}
                _ => {
                    bad_headers = true;
                    break;
                }
            }
        }

        if bad_headers
            && self
                .headers
                .get("ignore-invalid-headers")
                .unwrap_or(&Value::Bool(false))
                != &Value::Bool(true)
        {
            return Err(("Bad Request hea".to_string(), 400));
        }

        Ok(())
    }
}

fn main() {
    let stream = std::net::TcpListener::bind("localhost:8080").unwrap();

    for stream in stream.incoming() {
        let mut stream = stream.unwrap();

        std::thread::spawn(move || {

            println!("Handling connection from {}", stream.peer_addr().unwrap());
            let mut buffer = [0; 2048];
            let bytes_read = stream.read(&mut buffer).unwrap();
            let client_data = String::from_utf8_lossy(&buffer[..bytes_read]);

            let request: Option<JsontpRequest> = serde_json::from_str(&client_data).ok();

            let response = match request {
                Some(request) => match request.validate() {
                    Ok(_) => {
                        let file = std::fs::read_to_string(&request.resource);

                        let mut headers = HashMap::new();

                        headers.insert("date".to_string(), Value::String("".to_string()));
                        headers.insert("language".to_string(), Value::String("en-GB".to_string()));

                        JsontpResponse {
                            jsontp: "1.0".to_string(),
                            type_of_response: "response".to_string(),
                            status: match file {
                                Ok(_) => Status {
                                    code: 200,
                                    formal_message: "OK".to_string(),
                                    human_message: "Request was successful".to_string(),
                                },
                                Err(_) => Status {
                                    code: 404,
                                    formal_message: "Not Found".to_string(),
                                    human_message: "Resource not found".to_string(),
                                },
                            },
                            resource: request.resource,
                            headers: headers,
                            body: Body {
                                content: match file {
                                    Ok(content) => content,
                                    Err(_) => "".to_string(),
                                },
                                encoding: "identity".to_string(),
                                other: HashMap::new(),
                            },
                        }
                    }
                    Err((message, code)) => JsontpResponse {
                        jsontp: "1.0".to_string(),
                        type_of_response: "response".to_string(),
                        status: Status {
                            code,
                            formal_message: message.clone(),
                            human_message: message,
                        },
                        resource: request.resource,
                        headers: request.headers,
                        body: request.body,
                    },
                },
                None => JsontpResponse {
                    jsontp: "1.0".to_string(),
                    type_of_response: "response".to_string(),
                    status: Status {
                        code: 400,
                        formal_message: "Bad Request".to_string(),
                        human_message: "Request was not a valid JSONTP request".to_string(),
                    },
                    resource: "".to_string(),
                    headers: HashMap::new(),
                    body: Body {
                        content: "".to_string(),
                        encoding: "".to_string(),
                        other: HashMap::new(),
                    },
                },
            };

            let str_response = serde_json::to_string(&response).unwrap();

            stream.write(str_response.as_bytes()).unwrap();

            println!("handled connection from {}", stream.peer_addr().unwrap());
        });
    }
}
