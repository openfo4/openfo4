use proc_macro::TokenStream;

mod crash_handler;

#[proc_macro_attribute]
pub fn crash_handler_main(attr: TokenStream, input: TokenStream) -> TokenStream {
    crash_handler::crash_handler_main(attr, input)
}
