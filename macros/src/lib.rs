mod shader_data;

#[proc_macro_derive(ShaderData)]
pub fn derive_shader_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shader_data::expand_shader_data(syn::parse(input).unwrap())
}
