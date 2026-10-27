use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub entry: Option<String>,
    pub detail: Value,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Problem {
    pub count: usize,
    pub samples: Vec<Sample>,
}

/// Sorted problem kinds, with at most five samples in encounter order.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct Problems(pub BTreeMap<String, Problem>);

impl Problems {
    pub fn add(&mut self, kind: &str, entry: Option<&str>, detail: impl Into<Value>) {
        let item = self.0.entry(kind.to_owned()).or_default();
        item.count += 1;
        if item.samples.len() < 5 {
            item.samples.push(Sample {
                entry: entry.map(str::to_owned),
                detail: detail.into(),
            });
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub output: String,
    pub bytes: u64,
    pub entries: usize,
    pub body_bytes: usize,
    pub lookup_rows: usize,
    pub summaries: usize,
    pub resources: usize,
    pub links: usize,
    pub resource_refs: usize,
    pub meta: BTreeMap<String, String>,
    pub problems: Problems,
}
