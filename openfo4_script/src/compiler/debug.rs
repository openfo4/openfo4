use core::range::Range;
use std::fmt::Write;
use std::io;

use capstone::Capstone;
use capstone::arch::BuildsCapstone;
use capstone::arch::x86::ArchMode;
use openfo4_jit::{CompiledFunction, SourceLoc};

const TEMPLATE: &str = include_str!("debug.html");

#[derive(Clone, Debug)]
pub struct DebugInfo {
    pub name: String,
    pub pex: String,
    pub hir: HirDebugInfo,
}

#[derive(Clone, Debug)]
pub struct HirDebugInfo {
    pub text: String,
    pub regions: Vec<HirRegion>,
}

#[derive(Copy, Clone, Debug)]
pub struct HirRegion {
    pub region: Range<usize>,
    pub loc: Option<SourceLoc>,
}

pub fn write_debug_output(info: &DebugInfo, func: &CompiledFunction) {
    let disasm = Capstone::new()
        .x86()
        .mode(ArchMode::Mode64)
        .build()
        .unwrap();

    let mut hir = String::new();
    for region in &info.hir.regions {
        if region.loc.is_some() {
            write!(
                hir,
                "<span class=\"line\">{}</span>",
                &info.hir.text[region.region]
            )
            .unwrap();
        } else {
            write!(hir, "<span>{}</span>", &info.hir.text[region.region]).unwrap();
        }
    }

    let mut asm_to_hir = Vec::new();

    let mut asm = String::new();
    let base_addr = func.as_ptr().addr();

    let mut last_srcloc_end = 0;
    for (pos, srcloc) in func.srclocs().iter() {
        if pos.start > last_srcloc_end {
            let addr = base_addr + last_srcloc_end;
            let unmapped_region = last_srcloc_end..pos.start;
            let res = disasm
                .disasm_all(&func.as_bytes()[unmapped_region], addr as u64)
                .unwrap();
            write!(asm, "<span>{}</span>", res).unwrap();
        }

        let index = info
            .hir
            .regions
            .iter()
            .filter_map(|v| v.loc)
            .position(|loc| loc == srcloc)
            .unwrap();

        let addr = base_addr + pos.start;
        let res = disasm
            .disasm_all(&func.as_bytes()[pos], addr as u64)
            .unwrap();
        write!(asm, "<span class=\"line\">{}</span>", res).unwrap();

        asm_to_hir.push(index);
        last_srcloc_end = pos.end;
    }

    let mut hir_to_asm = vec![Vec::new(); info.hir.regions.len()];
    for (asm_index, hir_index) in asm_to_hir.iter().enumerate() {
        hir_to_asm[*hir_index].push(asm_index);
    }

    if last_srcloc_end != func.as_bytes().len() {
        let addr = base_addr + last_srcloc_end;
        let unmapped_region = last_srcloc_end..;
        let res = disasm
            .disasm_all(&func.as_bytes()[unmapped_region], addr as u64)
            .unwrap();
        write!(asm, "<span>{}</span>", res).unwrap();
    }

    let output = TEMPLATE
        .replace("{%NAME%}", &info.name)
        .replace("{%PEX%}", &info.pex)
        .replace("{%HIR%}", &hir)
        .replace("{%ASM%}", &asm)
        .replace(
            "{% HIR_TO_ASM %}",
            &hir_to_asm
                .iter()
                .map(|v| {
                    format!(
                        "[{}]",
                        v.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        )
        .replace(
            "{% ASM_TO_HIR %}",
            &asm_to_hir
                .iter()
                .map(|v| format!("[{v}]"))
                .collect::<Vec<_>>()
                .join(","),
        );

    if let Err(err) = write_output_file(&info.name, output.as_bytes()) {
        tracing::error!("failed to write output for {}: {}", info.name, err);
    }
}

fn write_output_file(name: &str, bytes: &[u8]) -> Result<(), io::Error> {
    let mut path = std::env::temp_dir();
    path.push("openfo4");
    path.push("script");
    std::fs::create_dir_all(&path)?;

    path.push(format!("{}.html", name));
    std::fs::write(&path, bytes)?;

    tracing::info!(
        "debug output for {} written to {}",
        name,
        path.to_string_lossy()
    );

    Ok(())
}
