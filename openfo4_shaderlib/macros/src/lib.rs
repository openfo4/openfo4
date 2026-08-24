use std::path::{Path, PathBuf};
use std::process::Command;

use game_render::shader::Shader;
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
    let config = slangc::Config {
        opt_level: slangc::OptLevel::Max,
        debug: false,
    };

    let bytes = slangc::compile(&path, &config).unwrap();
    // Validate that the shader loads correctly.
    let _ = Shader::from_spirv(&bytes).unwrap();
    bytes
}
