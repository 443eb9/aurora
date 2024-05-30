mod material_object;
mod shader_data;

#[proc_macro_derive(ShaderData)]
pub fn derive_shader_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    shader_data::expand_shader_data(syn::parse(input).unwrap())
}

#[proc_macro_derive(MaterialObject)]
pub fn derive_material_object(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    material_object::expand_material_object(syn::parse(input).unwrap())
}
