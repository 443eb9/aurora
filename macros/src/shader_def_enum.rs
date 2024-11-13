const DEF_NAME: &str = "def_name";

pub fn expand_shader_def_enum(input: syn::DeriveInput) -> proc_macro::TokenStream {
    let ty = &input.ident;

    let syn::Data::Enum(defs) = &input.data else {
        panic!()
    };

    let mut arms = Vec::with_capacity(defs.variants.len());

    for var in &defs.variants {
        let ident = &var.ident;
        if let Some(name) = var
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == DEF_NAME)
        {
            match &name.meta {
                syn::Meta::NameValue(name) => {
                    let def = &name.value;
                    arms.push(quote::quote! {
                        Self::#ident => #def.to_string(),
                    });
                }
                _ => panic!(),
            }
        } else {
            let def = variant_to_def(var.ident.to_string());
            arms.push(quote::quote! {
                Self::#ident => #def.to_string(),
            });
        }
    }

    quote::quote! {
        impl aurora_core::render::ShaderDefEnum for #ty {
            fn to_def(&self) -> (String, naga_oil::compose::ShaderDefValue) {
                (
                    match &self {
                        #(#arms)*
                    },
                    naga_oil::compose::ShaderDefValue::Bool(true),
                )
            }
        }
    }
    .into()
}

fn variant_to_def(var: String) -> String {
    let mut def = String::with_capacity(var.len());

    for c in var.chars() {
        if c.is_ascii_uppercase() && !def.is_empty() {
            def.push('_');
        }
        def.push(c.to_ascii_uppercase());
    }

    def
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_variant_to_def() {
        assert_eq!(variant_to_def("ExampleDef".to_string()), "EXAMPLE_DEF");
        assert_eq!(variant_to_def("Other".to_string()), "OTHER");
    }
}
