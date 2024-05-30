pub fn expand_shader_data(input: syn::DeriveInput) -> proc_macro::TokenStream {
    let package = std::env::var("CARGO_PKG_NAME").unwrap();
    let path = {
        if package == "aurora_core" {
            quote::quote! { crate }
        } else {
            quote::quote! { aurora_core }
        }
    };
    let ty = &input.ident;

    quote::quote! {
        impl #path::render::ShaderData for #ty {

        }
    }
    .into()
}
