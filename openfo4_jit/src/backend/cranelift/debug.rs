use std::fmt::Write;

use capstone::arch::BuildsCapstone;
use capstone::{Capstone, CsResult, Insn, arch};

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

pub fn build_disasm(buf: &[u8], srclocs: &SourceLocMap) -> TextWithSrcLocs<String> {
    let mut text = TextWithSrcLocs {
        text: String::new(),
        srclocs: SourceLocMap::default(),
    };

    let cs = create_capstone().unwrap();

    let mut last_range_end = 0;
    for (range, loc) in srclocs.iter() {
        if range.start > last_range_end {
            let bytes = &buf[last_range_end..range.start];
            let insts = cs.disasm_all(bytes, bytes.as_ptr().addr() as u64).unwrap();
            write_instructions(&mut text.text, &insts);
        }

        let srcloc_start = text.text.len();

        let bytes = &buf[range];
        let insts = cs.disasm_all(bytes, bytes.as_ptr().addr() as u64).unwrap();
        write_instructions(&mut text.text, &insts);

        let srcloc_end = text.text.len();

        text.srclocs.insert(srcloc_start, srcloc_end, loc);

        last_range_end = range.end;
    }

    // Trailing instructions.
    let bytes = &buf[last_range_end..];
    let insts = cs.disasm_all(bytes, bytes.as_ptr().addr() as u64).unwrap();
    write_instructions(&mut text.text, &insts);

    text
}

fn create_capstone() -> CsResult<Capstone> {
    let cs = Capstone::new();
    cfg_select! {
        target_arch = "x86" => cs.x86().mode(arch::x86::ArchMode::Mode32).build(),
        target_arch = "x86_64" => cs.x86().mode(arch::x86::ArchMode::Mode64).build(),
        _ => todo!("disassembly is not yet implemented for {}", std::env::consts::ARCH),
    }
}

fn write_instructions(buf: &mut String, insts: &[Insn<'_>]) {
    for inst in insts {
        writeln!(buf, "{}", inst).unwrap();
    }
}
