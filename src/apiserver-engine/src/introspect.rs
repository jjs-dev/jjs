use std::collections::HashMap;
#[derive(serde::Serialize)]
pub struct Introspection {
    pub components: HashMap<String, serde_json::Value>,
}

struct Introspector {
    gen: schemars::gen::SchemaGenerator,
}

impl Introspector {
    fn new() -> Introspector {
        let mut settings = schemars::gen::SchemaSettings::openapi3();
        settings.meta_schema = None;
        settings.definitions_path = "#/components/schemas/".to_string();
        Introspector {
            gen: settings.into_generator(),
        }
    }

    fn add_object<T: crate::api::ApiObject>(&mut self) -> &mut Self {
        let schema = self.gen.subschema_for::<T>();
        assert!(schema.is_ref());
        let name = <T as crate::api::ApiObject>::name().to_string();
        let qual_ty_name = std::any::type_name::<T>();
        let ty_name = qual_ty_name.rsplit("::").next().unwrap();
        assert_eq!(name, ty_name);
        self
    }

    fn into_introspection(self) -> Introspection {
        let defs = self.gen.into_definitions();
        let mut intro = Introspection {
            components: HashMap::default(),
        };
        for (name, schema) in defs {
            let schema =
                serde_json::to_value(&schema).expect("failed to serialize schema component");
            intro.components.insert(name, schema);
        }

        intro
    }
}

impl Introspection {}

pub fn introspect() -> Introspection {
    let mut introspector = Introspector::new();
    introspector
        .add_object::<crate::api::auth::SessionToken>()
        .add_object::<crate::api::auth::SimpleAuthParams>()
        .add_object::<crate::api::contests::Contest>()
        .add_object::<crate::api::contests::Problem>()
        .add_object::<crate::api::contests::Participation>()
        .add_object::<crate::api::misc::ApiVersion>()
        .add_object::<crate::api::runs::Run>()
        .add_object::<crate::api::runs::InvokeStatus>()
        .add_object::<crate::api::runs::RunPatch>()
        .add_object::<crate::api::runs::RunLiveStatusUpdate>()
        .add_object::<crate::api::runs::RunSimpleSubmitParams>()
        .add_object::<crate::api::toolchains::Toolchain>()
        .add_object::<crate::api::users::User>()
        .add_object::<crate::api::users::UserCreateParams>();

    introspector.into_introspection()
}
