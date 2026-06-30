use std::fmt::Write;

use crate::backend::TextWithSrcLocs;
use crate::{SourceLoc, SourceLocMap};

pub fn format_clif(clif: &str) -> TextWithSrcLocs<String> {
    let mut output = String::new();
    let mut srclocs = SourceLocMap::default();

    for line in clif.lines() {
        let mut line = line.trim();

        let mut srcloc = None;
        if line.starts_with("@") {
            let end = line.find(|c: char| c.is_whitespace()).unwrap();
            srcloc = Some(u32::from_str_radix(&line[1..end], 16).unwrap());
            line = line[end..].trim();
        }

        let indent = if line.starts_with("function") || line == "}" {
            0
        } else if line.starts_with("sig") || line.starts_with("fn") || line.starts_with("block") {
            2
        } else {
            // Normal instruction
            4
        };

        let start = output.len();

        for _ in 0..indent {
            output.push(' ');
        }

        writeln!(output, "{}", line).unwrap();

        let end = output.len();
        if let Some(loc) = srcloc {
            srclocs.insert(start, end, SourceLoc(loc));
        }
    }

    TextWithSrcLocs {
        text: output,
        srclocs,
    }
}
