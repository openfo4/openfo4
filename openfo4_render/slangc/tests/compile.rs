use slangc::Config;

#[test]
fn compile_test() {
    let input = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test.slang");
    slangc::compile(input, &Config::default()).unwrap();
}
