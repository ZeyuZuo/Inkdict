//! `zdict serve`: the desktop app's commands (apps/desktop/README.md) as
//! `POST /<command>` with the JSON arguments as body, on 127.0.0.1 only.
//! For developing the UI in a browser; the app itself uses Tauri.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server};
use zdict_app::{AppError, Dictionary, Settings};
use zdict_core::Link;

pub fn run(path: &Path, port: u16) -> Result<()> {
    let mut dictionary = Dictionary::open(path, Settings::default())
        .with_context(|| format!("opening {}", path.display()))?;
    dictionary.warm_up()?;
    let server = Server::http(("127.0.0.1", port))
        .map_err(|err| anyhow!("listening on port {port}: {err}"))?;
    eprintln!(
        "zdict: serving {} on http://127.0.0.1:{port}",
        path.display()
    );
    for mut request in server.incoming_requests() {
        let mut body = String::new();
        let (status, value) = if *request.method() == Method::Options {
            (204, Value::Null)
        } else if request.as_reader().read_to_string(&mut body).is_err() {
            (
                400,
                json!({"code": "bad_input", "message": "body is not UTF-8"}),
            )
        } else {
            let name = request.url().trim_start_matches('/').to_owned();
            let args: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            match call(&mut dictionary, &name, &args) {
                Ok(value) => (200, value),
                Err(err) => (400, serde_json::to_value(err)?),
            }
        };
        let response = Response::from_string(if status == 204 {
            String::new()
        } else {
            value.to_string()
        })
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json"))
        .with_header(header("Access-Control-Allow-Origin", "*"))
        .with_header(header("Access-Control-Allow-Headers", "content-type"));
        let _ = request.respond(response);
    }
    Ok(())
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("static header")
}

fn call(dictionary: &mut Dictionary, name: &str, args: &Value) -> Result<Value, AppError> {
    let text = || args["text"].as_str().unwrap_or_default().to_owned();
    let limit = || args["limit"].as_u64().unwrap_or(20) as usize;
    let to_json = |value: Result<Value, serde_json::Error>| {
        value.map_err(|err| AppError::new("internal", err.to_string()))
    };
    match name {
        "pack_info" => to_json(serde_json::to_value(dictionary.info())),
        "words" => to_json(serde_json::to_value(dictionary.words(&text(), limit())?)),
        "entry" => to_json(serde_json::to_value(
            dictionary.entry(args["key"].as_str().unwrap_or_default())?,
        )),
        "follow" => {
            let link: Link = serde_json::from_value(args["link"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            let current = args["current"].as_str().unwrap_or_default();
            to_json(serde_json::to_value(dictionary.follow(current, &link)?))
        }
        "settings" => to_json(serde_json::to_value(dictionary.settings())),
        "set_settings" => {
            let settings: Settings = serde_json::from_value(args["settings"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            to_json(serde_json::to_value(dictionary.set_settings(settings)))
        }
        "theme_css" => Ok(Value::String(dictionary.theme_css())),
        other => Err(AppError::new(
            "bad_input",
            format!("unknown command {other}"),
        )),
    }
}
