#!/usr/bin/env -S cargo +nightly -Zscript
```cargo
package.edition = "2021"
[dependencies]
anyhow = "1.0.79"
maplit = "1.0.2"
serde = { version = "1.0.194", features = ["derive"] }
serde_derive = "1.0.194"
serde_json = "1.0.111"
```
use anyhow::{bail, Result};
use maplit::hashmap;
use serde::Deserialize;

pub fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let file_data = std::fs::read_to_string(&args[1])?;
    let benchmark_name = &args[2];
    for line in file_data.lines() {
        let line = line.trim();
        if !line.starts_with("{") {
            continue
        }
        let perf: PerfJsonLine = serde_json::from_str(line)?;
        if let Some(event) = perf.event {
            if event == "task-clock" {
                let measure = "perf:task-clock";
                let value: f64 = perf.counter_value.unwrap().parse().unwrap();
                let hashmap = hashmap! {
                    benchmark_name => hashmap!{
                        measure => hashmap!{
                            "value" => value
                        }
                    }
                };
                let out = serde_json::to_string_pretty(&hashmap)?;
                println!("{out}");
                return Ok(());
            }
        }
    }

    bail!("instructions:u not found in input file");
}

#[derive(Debug, Deserialize)]
struct PerfJsonLine {
    #[serde(rename = "counter-value")]
    counter_value: Option<String>,

    //    unit: Option<String>,
    event: Option<String>,
    // #[serde(rename = "event-runtime")]
    // event_runtime: Option<u64>,

    // #[serde(rename = "pcnt-running")]
    // pcnt_running: Option<f64>,

    // #[serde(rename = "metric-value")]
    // metric_value: String,

    // #[serde(rename = "metric-unit")]
    // metric_unit: String,
}
