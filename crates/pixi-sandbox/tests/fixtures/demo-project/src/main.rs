//! Small on purpose: the point of this fixture is that `cargo vendor` has real crates to
//! carry, and that an offline `cargo build` after a restore proves the vendored tree works.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Ping {
    name: String,
}

fn main() {
    let ping = Ping {
        name: "demo-project".to_string(),
    };
    println!("{}", serde_json::to_string(&ping).expect("serialisable"));
}
