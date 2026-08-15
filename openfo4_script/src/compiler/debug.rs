use core::range::Range;
use std::fmt::Write;
use std::io;

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
    let mut hir = String::new();
    render_segmented_text(
        info.hir.regions.iter().map(|v| v.region),
        info.hir.text.len(),
        &mut hir,
        |region| *region,
        |buf, region| {
            write!(
                buf,
                "<span class=\"line\">{}</span>",
                &info.hir.text[*region]
            )
            .unwrap();
        },
        |buf, region| {
            write!(buf, "<span>{}</span>", &info.hir.text[region]).unwrap();
        },
    );

    let mut asm_to_hir = Vec::new();

    let mut asm = String::new();
    if let Some(disasm) = func.disasm() {
        render_segmented_text(
            disasm.srclocs.iter(),
            disasm.text.len(),
            &mut (&mut asm, &mut asm_to_hir),
            |(pos, _)| *pos,
            |(asm, asm_to_hir), (range, srcloc)| {
                let text = &disasm.text[*range];
                write!(asm, "<span class=\"line\">{}</span>", text).unwrap();

                let index = info
                    .hir
                    .regions
                    .iter()
                    .filter_map(|v| v.loc)
                    .position(|loc| loc == *srcloc)
                    .unwrap();

                asm_to_hir.push(index);
            },
            |(asm, _), range| {
                let text = &disasm.text[range];
                write!(asm, "<span>{}</span>", text).unwrap();
            },
        );
    }

    let mut hir_to_asm = vec![Vec::new(); info.hir.regions.len()];
    for (asm_index, hir_index) in asm_to_hir.iter().enumerate() {
        hir_to_asm[*hir_index].push(asm_index);
    }

    let mut clif = String::new();
    let mut clif_to_hir = Vec::new();
    if let Some(text) = func.text() {
        render_segmented_text(
            text.srclocs.iter(),
            text.text.len(),
            &mut (&mut clif, &mut clif_to_hir),
            |(range, _)| *range,
            |(clif, hir_to_clif), (range, srcloc)| {
                let text = &text.text[*range];
                write!(clif, "<span class=\"line\">{}</span>", text).unwrap();

                let index = info
                    .hir
                    .regions
                    .iter()
                    .filter_map(|v| v.loc)
                    .position(|loc| loc == *srcloc)
                    .unwrap();

                hir_to_clif.push(index);
            },
            |(clif, _), range| {
                let text = &text.text[range];
                write!(clif, "<span>{}</span>", text).unwrap();
            },
        )
    }

    let mut hir_to_clif = vec![Vec::new(); info.hir.regions.len()];
    for (clif_index, hir_index) in clif_to_hir.iter().enumerate() {
        hir_to_clif[*hir_index].push(clif_index);
    }

    let output = TEMPLATE
        .replace("{%NAME%}", &info.name)
        .replace("{%PEX%}", &info.pex)
        .replace("{%HIR%}", &hir)
        .replace("{%ASM%}", &asm)
        .replace("{%CLIF%}", &clif)
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
        )
        .replace(
            "{% HIR_TO_CLIF %}",
            &hir_to_clif
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
            "{% CLIF_TO_HIR %}",
            &clif_to_hir
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

fn render_segmented_text<T, U>(
    segments: impl Iterator<Item = U>,
    len: usize,
    buf: &mut T,
    get_range: impl Fn(&U) -> Range<usize>,
    mut render_segment: impl FnMut(&mut T, &U),
    mut render_unmapped: impl FnMut(&mut T, Range<usize>),
) {
    let mut last_index = 0;

    for segment in segments {
        let range = get_range(&segment);

        if range.start > last_index {
            render_unmapped(
                buf,
                Range {
                    start: last_index,
                    end: range.start,
                },
            );
        }

        render_segment(buf, &segment);
        last_index = range.end;
    }

    if last_index != len {
        render_unmapped(
            buf,
            Range {
                start: last_index,
                end: len,
            },
        );
    }
}
