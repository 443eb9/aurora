pub fn expand_shader_data(input: syn::DeriveInput) -> proc_macro::TokenStream {
    let ty = &input.ident;

    quote::quote! {
        impl crate::render::ShaderData for #ty {

        }
    }
    .into()
}
