//! Proc-macros for the rSolidity contract vocabulary.
//!
//! This crate is an implementation detail of `hkask-rsolidity`. Public API users
//! should depend on `hkask-rsolidity` and use its re-exports.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, ExprLit, FnArg, ItemFn, Lit, Meta, MetaNameValue, Stmt, Token,
    parse::{Parser, Result as ParseResult},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
};

/// Parse a comma-separated list of `name = "value"` meta items.
fn parse_name_value_args(args: TokenStream) -> ParseResult<Vec<MetaNameValue>> {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse(args)?;
    let mut pairs = Vec::new();
    for meta in metas {
        if let Meta::NameValue(nv) = meta {
            pairs.push(nv);
        }
    }
    Ok(pairs)
}

fn string_lit(nv: &MetaNameValue) -> ParseResult<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = &nv.value
    {
        Ok(s.value())
    } else {
        Err(syn::Error::new_spanned(
            &nv.value,
            "expected string literal",
        ))
    }
}

fn get_named_value<'a>(pairs: &'a [MetaNameValue], name: &str) -> Option<&'a MetaNameValue> {
    pairs.iter().find(|nv| nv.path.is_ident(name))
}

/// REQ: P9-rsolidity-macros-ocap
/// pre:  arguments are valid
/// post: returns expected result
/// Injects a capability check at the start of the annotated method. The
/// receiver type must implement `::hkask_rsolidity::Ocap`.
#[proc_macro_attribute]
pub fn ocap(args: TokenStream, input: TokenStream) -> TokenStream {
    let pairs = match parse_name_value_args(args) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    let resource = match get_named_value(&pairs, "resource") {
        Some(nv) => match string_lit(nv) {
            Ok(s) => s,
            Err(e) => return e.to_compile_error().into(),
        },
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "ocap requires `resource = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };

    let operation = match get_named_value(&pairs, "operation") {
        Some(nv) => match string_lit(nv) {
            Ok(s) => s,
            Err(e) => return e.to_compile_error().into(),
        },
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "ocap requires `operation = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut item = parse_macro_input!(input as ItemFn);
    let has_self = item
        .sig
        .inputs
        .first()
        .is_some_and(|arg| matches!(arg, FnArg::Receiver(_)));
    if !has_self {
        return syn::Error::new_spanned(
            &item.sig,
            "ocap attribute requires a method with a `self` receiver",
        )
        .to_compile_error()
        .into();
    }

    let check: Stmt = parse_quote! {
        <Self as ::hkask_rsolidity::Ocap>::verify_ocap(self, #resource, #operation)?;
    };
    item.block.stmts.insert(0, check);
    quote!(#item).into()
}

/// REQ: P9-rsolidity-macros-contract
/// pre:  arguments are valid
/// post: returns expected result
/// Compile-time contract metadata. Validates the contract ID and principle
/// format, then re-emits the annotated item unchanged so the existing source
/// REQ comments remain the authoritative audit signal.
#[proc_macro_attribute]
pub fn contract(args: TokenStream, input: TokenStream) -> TokenStream {
    let pairs = match parse_name_value_args(args) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    let id_nv = match get_named_value(&pairs, "id") {
        Some(nv) => nv,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "contract requires `id = \"...\"`",
            )
            .to_compile_error()
            .into();
        }
    };
    let id = match string_lit(id_nv) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    let principle_nv = match get_named_value(&pairs, "principle") {
        Some(nv) => nv,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "contract requires `principle = \"P#\"`",
            )
            .to_compile_error()
            .into();
        }
    };
    let principle = match string_lit(principle_nv) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    // P{N}-... format (N can be 1-2 digits: P1-P12)
    let dash_pos = id.find('-');
    let id_ok = if let Some(pos) = dash_pos {
        pos >= 2
            && pos <= 3
            && id.starts_with('P')
            && id[1..pos].chars().all(|c| c.is_ascii_digit())
    } else {
        false
    };
    if !id_ok {
        return syn::Error::new_spanned(id_nv, format!("contract id `{}` must match `P#-...`", id))
            .to_compile_error()
            .into();
    }

    let principle_ok =
        principle.len() > 1 && principle.starts_with('P') && principle[1..].parse::<u8>().is_ok();
    if !principle_ok {
        return syn::Error::new_spanned(
            principle_nv,
            format!("principle `{}` must be P1-P12", principle),
        )
        .to_compile_error()
        .into();
    }

    let item = parse_macro_input!(input as ItemFn);
    quote!(#item).into()
}
