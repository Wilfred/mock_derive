/*
MIT License

Copyright (c) 2017 David DeSimone

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

#![feature(proc_macro)]
#![recursion_limit = "128"]

//#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use std::str::FromStr;

struct Function {
    pub name: syn::Ident,
    pub decl: syn::FnDecl
}

fn parse_impl(item: &syn::Item) -> Vec<Function> {
    let mut result = Vec::new();
    match item.node {
        syn::ItemKind::Impl(_unsafety, _impl_token, ref _generics, ref _trait_, ref _self_ty, ref items) => {
            for item in items {
                match item.node {
                    syn::ImplItemKind::Method(ref sig, ref _block) => {
                        result.push(Function {name: item.ident.clone(), decl: sig.decl.clone() } );
                    },
                    _ => { }
                }
            }
        },
        _ => { panic!("#[mock] must be applied to an Impl statement."); }
    };

    result
}

#[proc_macro_attribute]
pub fn mock(_attr_ts: TokenStream, impl_ts: TokenStream) -> TokenStream {
    let impl_item = syn::parse_item(&impl_ts.to_string()).unwrap();

    let fns = parse_impl(&impl_item);
    let mut methods = quote::Tokens::new();
    
    // For each method in the Impl block, we create a "method_" name function that returns an
    // object to mutate
    for fnc in fns {
        let name = fnc.name;
        let decl = fnc.decl.inputs;
        let name_stream = quote! { #name };
        let ident = concat_idents("method_", name_stream.as_str());
        let mut args = Vec::new();
        for input in decl {
            match input {
                syn::FnArg::Captured(_pat, ty) => {
                    args.push(ty);
                },
                _ => {}
            }
        }

        let _ = match args.len() {
            0 => { quote!{ () } },
            1 => { quote!{ (#args[0]) } },
            2 => { quote!{ (#args[0], #args[1]) } },
            3 => { quote!{ (#args[0], #args[1], #args[2]) } },
            _ => { panic!("Unexpected number of args, max is 3") }
        };

        let return_type = match fnc.decl.output {
            syn::FunctionRetTy::Default => { quote! { () } },
            syn::FunctionRetTy::Ty(ref ty) => { quote! { #ty } },
        };
        
        methods = quote! {
            #methods
            pub fn #ident(&mut self) -> MockMethod<T, #return_type> {
                MockMethod { imp: self, retval: std::collections::HashMap::new() }
            }
        }
    }    
    
    let stream = quote! {
        // @TODO make unique name
        struct MockImpl<T> {
            fallback: Option<T>,
            call_num: usize
        }

        // @TODO add impl block that adds mock functionality
        // @TODO this hashmap is mapping call to ARG values,
        // not call to RETURN value. We only care about args when generating the mock
        // function.
        struct MockMethod<'a, T: 'a, U> {
            imp: &'a mut MockImpl<T>,
            retval: std::collections::HashMap<usize, U>,
        }

        impl<T> MockImpl<T> {
            #methods

            pub fn new() -> MockImpl<T> {
                MockImpl { fallback: None, call_num: 0 }
            }

            pub fn set_fallback(&mut self, t: T) {
                self.fallback = Some(t);
            }
        }

        impl<'a, T: 'a, U> MockMethod<'a, T, U> {
            pub fn first_call(mut self) -> Self {
                self.nth_call(1)
            }

            pub fn second_call(mut self) -> Self {
                self.nth_call(2)
            }

            pub fn nth_call(mut self, num: usize) -> Self {
                self.imp.call_num = num;
                self
            }

            pub fn set_result(mut self, retval: U) -> Self {
                self.retval.insert(self.imp.call_num, retval);
                self
            }

            pub fn when<F>(mut self, _: F) -> Self
                where F: FnOnce() -> bool {
                self
            }
        }

        impl<T> HelloWorld for MockImpl<T> {
            fn hello_world(&self) {
                println!("World Hello");
            }
        }
    };

    TokenStream::from_str(stream.as_str()).unwrap()
}

fn concat_idents(lhs: &str, rhs: &str) -> syn::Ident {
    syn::Ident::new(format!("{}{}", lhs, rhs))
}
