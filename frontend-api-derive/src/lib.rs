extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};

fn process_query(name: &str) -> TokenStream {
    let path = format!("./queries/{}.graphql", name);
    let struct_name = syn::Ident::new(name, proc_macro2::Span::call_site());
    quote::quote!(
         #[derive(GraphQLQuery)]
         #[graphql(
            schema_path = "../frontend-api/src/schema-gen.json",
            query_path = #path,
            response_derives = "Serialize"
        )]
        pub struct #struct_name;
    )
    .into()
}

#[proc_macro]
pub fn define_query(item: TokenStream) -> TokenStream {
    let iter = item.into_iter();
    let mut out = TokenStream::new();
    for tok in iter {
        match tok {
            TokenTree::Ident(ident) => {
                let s = ident.to_string();
                let ts = process_query(&s);
                out.extend(ts);
            }
            _ => panic!("unexpected token: {}", tok),
        }
    }
    out
}
