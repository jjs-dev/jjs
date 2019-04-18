use proc_macro2::{Ident, Span};
use quote::quote;
use std::io::Write;
#[derive(Debug)]
struct ApiCallInfo {
    name: String,
    arg_type: syn::Type,
    ret_type: syn::Type,
}

impl ApiCallInfo {
    fn new(meth: &syn::TraitItemMethod) -> Self {
        let name = meth.sig.ident.to_string();
        let sig = &meth.sig.decl;
        let ret_type = match sig.output {
            syn::ReturnType::Default => panic!("expected return type"),
            syn::ReturnType::Type(_arrow, ref type_box) => type_box.as_ref(),
        };

        let arg_type = sig.inputs.iter().collect::<Vec<_>>();
        assert_eq!(arg_type.len(), 1);
        let arg_type = arg_type[0];
        let arg_type = match arg_type {
            syn::FnArg::Captured(arg) => arg,
            _ => panic!("unexpected parameter kind"),
        };

        ApiCallInfo {
            name,
            arg_type: arg_type.ty.clone(),
            ret_type: ret_type.clone(),
        }
    }

    fn to_method_def(&self) -> String {
        let self_name = Ident::new(&self.name, Span::call_site());
        let self_arg_type = &self.arg_type;
        let self_ret_type = &self.ret_type;
        let self_url = self_name.to_string().replace('_', "/");
        (quote! {
            impl Client {
                pub fn #self_name(&self, params: &#self_arg_type) -> Result<#self_ret_type, reqwest::Error> {
                    self.exec_query(#self_url, params)
                }
            }
        })
        .to_string()
    }
}

fn main() {
    let file = std::fs::read("./src/typings.rs").unwrap();
    let file = String::from_utf8(file).unwrap();
    let out_file_path = format!("{}/client_gen.rs", std::env::var("OUT_DIR").unwrap());
    println!("emitting to {}", &out_file_path);
    let mut out_file = std::fs::File::create(&out_file_path).unwrap();
    let ast = syn::parse_file(&file).expect("Parse error");
    let mut api_trait = None;
    for item in &ast.items {
        if let syn::Item::Trait(tr) = item {
            let tr_name = format!("{}", tr.ident);
            if tr_name == "Frontend" {
                api_trait = Some(tr);
                break;
            }
        } else {
            let def = quote! {
                #item
            };
            let mut def = def.to_string();
            def.push('\n');
            out_file.write_all(def.as_bytes()).unwrap();
        }
    }
    let api_trait = api_trait.expect("Couldn't find Frontend trait");

    let mut api_call_info = Vec::new();

    for it in &api_trait.items {
        let it = match it {
            syn::TraitItem::Method(me) => me,
            oth => panic!("Unexpected item: {:?}", oth),
        };
        api_call_info.push(ApiCallInfo::new(it));
    }
    let api_meth_defs = api_call_info
        .iter()
        .map(|api_call_info| api_call_info.to_method_def())
        .collect::<Vec<_>>();
    for def in api_meth_defs {
        out_file.write_all(def.as_bytes()).unwrap();
        out_file.write_all("\n".as_bytes()).unwrap();
    }
}
