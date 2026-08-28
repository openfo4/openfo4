use std::path::{Path, PathBuf};
use std::process::Command;

use game_render::shader::Shader;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{LitByteStr, LitStr, parse_macro_input};

#[proc_macro]
pub fn include_shader(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr).value();
    let input = input.strip_prefix('/').unwrap_or(&input);

    let path = workspace_dir().join(input);

    let bytes = compile(&path);
    let bytes = LitByteStr::new(&bytes, Span::call_site());

    TokenStream::from(quote! {
        openfo4_shaderlib::ShaderSource::from_spirv(#bytes)
    })
}

fn workspace_dir() -> PathBuf {
    let output = Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

fn compile(path: &Path) -> Vec<u8> {
    let bytes = if path.extension().is_some_and(|ext| ext == "slang") {
        compile_slang(path)
    } else if path.extension().is_some_and(|ext| ext == "wgsl") {
        compile_wgsl(path)
    } else {
        panic!("unknown shader extension: {:?}", path.extension())
    };

    // Validate that the shader loads correctly.
    let _ = Shader::from_spirv(&bytes).unwrap();
    bytes
}

fn compile_slang(path: &Path) -> Vec<u8> {
    let config = slangc::Config {
        opt_level: slangc::OptLevel::Max,
        debug: false,
    };

    let bytes = slangc::compile(&path, &config).unwrap();
    bytes
}

fn compile_wgsl(path: &Path) -> Vec<u8> {
    let data = std::fs::read_to_string(path).unwrap();
    let module = naga::front::wgsl::parse_str(&data).unwrap();
    let info = Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap();
    let words = naga::back::spv::write_vec(&module, &info, &Default::default(), None).unwrap();
    words
        .into_iter()
        .map(|word| word.to_le_bytes())
        .flatten()
        .collect()
}
