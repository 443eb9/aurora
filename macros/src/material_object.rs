pub fn expand_material_object(input: syn::DeriveInput) -> proc_macro::TokenStream {
    let ty = &input.ident;

    quote::quote! {
        impl aurora_core::scene::SceneObject for #ty {
            fn insert_self(self, scene: &mut aurora_core::scene::Scene) -> Uuid {
                let ty = aurora_core::util::ext::TypeIdAsUuid::to_uuid(std::any::Any::type_id(&self));
                let uuid = uuid::Uuid::new_v4();
                scene.materials.insert(uuid, (Box::new(self), ty));
                scene.asset_events.push(aurora_core::scene::AssetEvent::Added(uuid, aurora_core::scene::AssetType::Material));
                uuid
            }
        }
    }
    .into()
}
