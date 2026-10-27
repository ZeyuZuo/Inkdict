#![allow(dead_code)]
use inkdict_pack::{Options, Report};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

pub const TYPES: &str = r##"
[meta]
format = "inkdict-types"
version = 1
[groups]
all = ["#text", "word", "variant", "phrase", "infl", "reading", "hidden", "group", "hom", "choice", "optional", "link", "media", "badge", "required"]
[types.entry]
kind = "block"
children = ["@all"]
[types.word]
kind = "inline"
roles = ["index"]
index = "headword"
children = ["@all"]
[types.variant]
kind = "inline"
roles = ["index"]
index = "variant"
children = ["#text"]
[types.phrase]
kind = "inline"
roles = ["index"]
index = "phrase"
children = ["#text"]
[types.infl]
kind = "inline"
roles = ["index"]
index = "inflection"
children = ["#text"]
[types.reading]
kind = "inline"
roles = ["index"]
index = "reading"
children = ["#text"]
[types.hidden]
kind = "inline"
roles = ["index"]
index = "hidden"
children = ["#text"]
[types.group]
kind = "block"
roles = ["anchor"]
children = ["@all"]
[types.hom]
kind = "inline"
roles = ["homograph"]
children = ["#text"]
[types.choice]
kind = "inline"
roles = ["choice"]
children = ["#text"]
[types.optional]
kind = "inline"
roles = ["optional"]
children = ["#text"]
[types.link]
kind = "inline"
roles = ["link"]
attrs = { entry = "ref?", anchor = "ref?" }
children = ["#text"]
[types.media]
kind = "inline"
attrs = { z = "res?", a = "res?" }
[types.badge]
kind = "inline"
attrs = { level = "int?", flag = "bool?" }
[types.required]
kind = "inline"
attrs = { z = "string", a = "int", b = "bool", c = "enum(x|y)", d = "res", e = "ref" }
children = ["#text"]
"##;

pub struct Fixture {
    pub dir: TempDir,
}
impl Fixture {
    pub fn new(entries: &[Value]) -> Self {
        let f = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        f.write("types.toml", TYPES);
        f.write("theme.toml", "[meta]\nformat='inkdict-theme'\nversion=1\n");
        f.write("manifest.toml", "[meta]\nformat='inkdict-source'\nversion=1\n[pack]\nid='test'\ntitle='Test'\n[languages]\nsource='en'\ntargets=['zh-Hans', 'fr']\n[files]\ntypes='types.toml'\nthemes=['theme.toml']\nentries='entries.jsonl'\n[lookup]\nnorm='fold-1'\nboost=['badge[level=1]', 'badge[flag=true]']\n");
        f.write(
            "entries.jsonl",
            &entries
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        );
        f
    }
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
    pub fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    pub fn replace(&self, name: &str, from: &str, to: &str) {
        self.write(
            name,
            &fs::read_to_string(self.path(name))
                .unwrap()
                .replace(from, to),
        );
    }
    pub fn options(&self) -> Options {
        Options::new(self.path("manifest.toml"))
    }
    pub fn build(&self) -> Report {
        inkdict_pack::build(self.options()).unwrap()
    }
    pub fn db(&self) -> Connection {
        Connection::open(self.path("test.inkdict")).unwrap()
    }
    pub fn error(&self) -> String {
        inkdict_pack::build(self.options()).unwrap_err().to_string()
    }
}

pub fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/samples/en-zh")
}
