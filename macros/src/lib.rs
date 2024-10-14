mod shader_data;
mod shader_def_enum;

#[proc_macro_derive(ShaderData)]
pub fn derive_shader_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shader_data::expand_shader_data(syn::parse(input).unwrap())
}

#[proc_macro_derive(ShaderDefEnum, attributes(def_name))]
pub fn derive_shader_def_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shader_def_enum::expand_shader_def_enum(syn::parse(input).unwrap())
}
