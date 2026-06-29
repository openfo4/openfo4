use core::range::Range;
use std::fmt::Write;
use std::io;
use std::sync::LazyLock;

use capstone::arch::BuildsCapstone;
use capstone::arch::x86::ArchMode;
use capstone::{Capstone, Insn};
use hashbrown::HashMap;
use openfo4_jit::{CompiledFunction, SourceLoc};

use crate::runtime::host_functions;

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
    let base_addr = func.as_ptr().addr();

    render_segmented_text(
        func.srclocs().iter(),
        func.as_bytes().len(),
        &mut (&mut asm, &mut asm_to_hir),
        |(pos, _)| *pos,
        |(asm, asm_to_hir), (range, srcloc)| {
            let index = info
                .hir
                .regions
                .iter()
                .filter_map(|v| v.loc)
                .position(|loc| loc == *srcloc)
                .unwrap();

            let addr = base_addr + range.start;
            let res = disasm
                .disasm_all(&func.as_bytes()[*range], addr as u64)
                .unwrap();
            let res: String = res
                .iter()
                .map(|ins| format_instruction(ins))
                .collect::<Vec<_>>()
                .join("\n");
            write!(asm, "<span class=\"line\">{}\n</span>", res).unwrap();

            asm_to_hir.push(index);
        },
        |(asm, _), range| {
            let addr = base_addr + range.start;
            let res = disasm
                .disasm_all(&func.as_bytes()[range], addr as u64)
                .unwrap();
            let res: String = res
                .iter()
                .map(|ins| format_instruction(ins))
                .collect::<Vec<_>>()
                .join("\n");
            write!(asm, "<span>{}\n</span>", res).unwrap();
        },
    );

    let mut hir_to_asm = vec![Vec::new(); info.hir.regions.len()];
    for (asm_index, hir_index) in asm_to_hir.iter().enumerate() {
        hir_to_asm[*hir_index].push(asm_index);
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

fn format_instruction(ins: &Insn<'_>) -> String {
    #[allow(function_casts_as_integer)]
    static SYMBOLS: LazyLock<HashMap<usize, &'static str>> = LazyLock::new(|| {
        HashMap::from([
            (
                host_functions::call_host_trampoline as usize,
                "call_host_trampoline",
            ),
            (host_functions::alloc_zeroed as usize, "alloc_zeroed"),
            (
                host_functions::alloc_stack_frame as usize,
                "alloc_stack_frame",
            ),
            (
                host_functions::free_stack_frame as usize,
                "free_stack_frame",
            ),
            (host_functions::bool_to_string as usize, "bool_to_string"),
            (host_functions::i32_to_string as usize, "i32_to_string"),
            (host_functions::f32_to_string as usize, "f32_to_string"),
            (
                host_functions::object_to_string as usize,
                "object_to_string",
            ),
            (host_functions::enter_span as usize, "enter_span"),
            (host_functions::exit_span as usize, "exit_span"),
            (host_functions::stub as usize, "stub"),
        ])
    });

    let s = ins.to_string();

    let Some(imm_start) = s.trim().rfind("0x") else {
        return s;
    };

    let Ok(imm) = usize::from_str_radix(s[imm_start + 2..].trim(), 16) else {
        return s;
    };

    if let Some(symbol) = SYMBOLS.get(&imm) {
        format!("{s} # {symbol}")
    } else {
        s
    }
}
