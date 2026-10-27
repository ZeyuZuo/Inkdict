//! `inkdict serve`: the desktop app's commands (apps/desktop/README.md) as
//! `POST /<command>` with the JSON arguments as body, on 127.0.0.1 only;
//! `GET /resource/<pack id>/<kind>/<key>` serves files the pack stores.
//! For developing the UI in a browser; the app itself uses Tauri.

use std::path::Path;

use anyhow::{Result, anyhow};
use inkdict_app::{AppError, Dictionary, Session, Settings};
use inkdict_core::Link;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server};

pub fn run(path: &Path, port: u16) -> Result<()> {
    let (mut session, error) = Session::start(None, Some(path.to_owned()));
    session.set_resource_base(format!("http://127.0.0.1:{port}/resource/"));
    if let Some(error) = error {
        eprintln!("inkdict: {error}");
    }
    if let Some(dictionary) = session.dictionary() {
        dictionary.warm_up()?;
    }
    let server = Server::http(("127.0.0.1", port))
        .map_err(|err| anyhow!("listening on port {port}: {err}"))?;
    eprintln!(
        "inkdict: serving {} on http://127.0.0.1:{port}",
        path.display()
    );
    for mut request in server.incoming_requests() {
        if *request.method() == Method::Get
            && let Some(path) = request.url().strip_prefix("/resource/")
        {
            let found = session
                .dictionary()
                .and_then(|dictionary| dictionary.resource(path));
            let response = match found {
                Some((mime, body)) => {
                    Response::from_data(body).with_header(header("Content-Type", &mime))
                }
                None => Response::from_data(Vec::new()).with_status_code(404),
            };
            let _ =
                request.respond(response.with_header(header("Access-Control-Allow-Origin", "*")));
            continue;
        }
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
            match call(&mut session, &name, &args) {
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

fn call(session: &mut Session, name: &str, args: &Value) -> Result<Value, AppError> {
    let path = || -> Result<std::path::PathBuf, AppError> {
        serde_json::from_value(args["path"].clone())
            .map_err(|err| AppError::new("bad_input", err.to_string()))
    };
    let text = || args["text"].as_str().unwrap_or_default().to_owned();
    let limit = || args["limit"].as_u64().unwrap_or(20) as usize;
    let to_json = |value: Result<Value, serde_json::Error>| {
        value.map_err(|err| AppError::new("internal", err.to_string()))
    };
    match name {
        "pack_info" => to_json(serde_json::to_value(dictionary(session)?.info())),
        "library" => to_json(serde_json::to_value(session.library())),
        "add_packs" => {
            let paths = serde_json::from_value(args["paths"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            to_json(serde_json::to_value(session.add_packs(paths)?))
        }
        "remove_pack" => to_json(serde_json::to_value(session.remove_pack(&path()?)?)),
        "move_pack" => {
            let index: usize = serde_json::from_value(args["index"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            to_json(serde_json::to_value(session.move_pack(&path()?, index)?))
        }
        "use_pack" => to_json(serde_json::to_value(session.use_pack(&path()?)?)),
        "words" => to_json(serde_json::to_value(
            dictionary(session)?.words(&text(), limit())?,
        )),
        "entry" => to_json(serde_json::to_value(
            dictionary(session)?.entry(args["key"].as_str().unwrap_or_default())?,
        )),
        "follow" => {
            let link: Link = serde_json::from_value(args["link"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            let current = args["current"].as_str().unwrap_or_default();
            to_json(serde_json::to_value(
                dictionary(session)?.follow(current, &link)?,
            ))
        }
        "settings" => to_json(serde_json::to_value(session.settings())),
        "set_settings" => {
            let settings: Settings = serde_json::from_value(args["settings"].clone())
                .map_err(|err| AppError::new("bad_input", err.to_string()))?;
            to_json(serde_json::to_value(session.set_settings(settings)?))
        }
        "theme_css" => Ok(Value::String(dictionary(session)?.theme_css())),
        other => Err(AppError::new(
            "bad_input",
            format!("unknown command {other}"),
        )),
    }
}

fn dictionary(session: &Session) -> Result<&Dictionary, AppError> {
    session.dictionary().ok_or_else(AppError::no_pack)
}
