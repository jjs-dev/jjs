use std::path::PathBuf;

pub(crate) enum ItemKind {
    Bash,
    Cpp,
}

pub(crate) fn find_items(kind: ItemKind) -> impl Iterator<Item = PathBuf> {
    let mut types_builder = ignore::types::TypesBuilder::new();
    types_builder.add_defaults();
    types_builder.negate("all");
    match kind {
        ItemKind::Bash => {
            types_builder.select("sh");
        }
        ItemKind::Cpp => {
            types_builder.select("c");
            types_builder.select("cpp");
        }
    }
    let types_matched = types_builder.build().unwrap();
    ignore::WalkBuilder::new(".")
        .types(types_matched)
        .build()
        .map(Result::unwrap)
        .filter(|x| {
            let ty = x.file_type();
            match ty {
                Some(f) => f.is_file(),
                None => false,
            }
        })
        .map(|x| x.path().to_path_buf())
}
