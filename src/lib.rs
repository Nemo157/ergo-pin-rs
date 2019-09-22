//! <p align="center"><i>Immobilis ergo pin</i></p>
//!
//! **Ergo**nomic stack **pin**ning for Rust.
//!
//! `ergo-pin` exports a single proc-macro-attribute `#[ergo_pin]` that can be applied to a
//! item/block/`tt`-accepting-macro-invocation to provide the "magical" `pin!`
//! within the scope. You can consider this `pin!` macro equivalent to a function
//! with the signature:
//!
//! ```ignore
//! extern "bla̴ck̀ mag̸ic͘" fn pin!<T>(t: T) -> Pin<&'local mut T>;
//! ```
//!
//! it will take in any value and return a `Pin<&mut _>` of the value, with the
//! correct local stack lifetime.
//!
//! # Examples
//!
//! ## Pin values inside functions
//!
//! ```rust
//! use core::pin::Pin;
//! use ergo_pin::ergo_pin;
//!
//! struct Foo;
//!
//! impl Foo {
//!     fn foo(self: Pin<&mut Self>) -> usize {
//!         5
//!     }
//! }
//!
//! #[ergo_pin]
//! fn foo() -> usize {
//!     pin!(Foo).foo()
//! }
//!
//! assert_eq!(foo(), 5);
//! ```
//!
//! ## Pin values in blocks (requires unstable features)
//!
#![cfg_attr(feature = "nightly-tests", doc = "```rust")]
#![cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
//! #![feature(stmt_expr_attributes, proc_macro_hygiene)]
//!
//! use core::pin::Pin;
//! use ergo_pin::ergo_pin;
//!
//! struct Foo;
//!
//! impl Foo {
//!     fn foo(self: Pin<&mut Self>) -> usize {
//!         5
//!     }
//! }
//!
//! fn foo() -> usize {
//!     #[ergo_pin] {
//!         pin!(Foo).foo()
//!     }
//! }
//!
//! assert_eq!(foo(), 5);
//! ```
//!
//! ## Pin values in other macros that accept normal Rust code (requires unstable features)
//!
#![cfg_attr(feature = "nightly-tests", doc = "```rust")]
#![cfg_attr(not(feature = "nightly-tests"), doc = "```ignore")]
//! #![feature(proc_macro_hygiene)]
//!
//! use core::pin::Pin;
//! use ergo_pin::ergo_pin;
//!
//! struct Foo;
//!
//! impl Foo {
//!     fn foo(self: Pin<&mut Self>) -> usize {
//!         5
//!     }
//! }
//!
//! macro_rules! bar {
//!     ($($tokens:tt)+) => { $($tokens)+ };
//! }
//!
//! fn foo() -> usize {
//!     #[ergo_pin]
//!     bar! {
//!         pin!(Foo).foo()
//!     }
//! }
//!
//! assert_eq!(foo(), 5);
//! ```
//!
//! ## Pin values inside any function of an impl
//!
//! (Note: this does _not_ descend into macros of the inner code as they may not be using normal
//! Rust code syntax.)
//!
//! ```rust
//! use core::pin::Pin;
//! use ergo_pin::ergo_pin;
//!
//! struct Foo;
//!
//! impl Foo {
//!     fn foo(self: Pin<&mut Self>) -> usize {
//!         5
//!     }
//! }
//!
//! struct Bar;
//!
//! #[ergo_pin]
//! impl Bar {
//!     fn bar() -> usize {
//!         pin!(Foo).foo()
//!     }
//! }
//!
//! assert_eq!(Bar::bar(), 5);
//! ```

extern crate proc_macro;

use quote::{quote, ToTokens};
use syn::fold::Fold;

#[derive(Default)]
struct Visitor {
    counter: usize,
    pinned: Vec<(syn::Ident, syn::Expr)>,
}

impl Visitor {
    fn new() -> Self {
        Self::default()
    }

    fn gen_ident(&mut self) -> syn::Ident {
        let string = format!("__ergo_pin_{}", self.counter);
        self.counter += 1;
        syn::Ident::new(&string, proc_macro2::Span::call_site())
    }
}

impl Fold for Visitor {
    fn fold_block(&mut self, block: syn::Block) -> syn::Block {
        syn::Block {
            brace_token: block.brace_token,
            stmts: block
                .stmts
                .into_iter()
                .flat_map(|stmt| {
                    let prior = std::mem::replace(&mut self.pinned, vec![]);
                    let stmt = self.fold_stmt(stmt);
                    std::mem::replace(&mut self.pinned, prior)
                        .into_iter()
                        .flat_map(|(ident, expr)| {
                            syn::parse::<syn::Block>(
                                quote!({
                                    let mut #ident = #expr;
                                    let #ident = unsafe {
                                        ::core::pin::Pin::new_unchecked(&mut #ident)
                                    };
                                })
                                .into(),
                            )
                            .unwrap()
                            .stmts
                        })
                        .chain(std::iter::once(stmt))
                })
                .collect(),
        }
    }

    fn fold_expr(&mut self, expr: syn::Expr) -> syn::Expr {
        let pin = syn::Ident::new("pin", proc_macro2::Span::call_site());
        if let syn::Expr::Macro(expr) = expr {
            if expr.mac.path.is_ident(&pin) {
                let ident = self.gen_ident();
                self.pinned
                    .push((ident.clone(), syn::parse(expr.mac.tokens.into()).unwrap()));
                syn::Expr::Path(syn::ExprPath {
                    attrs: vec![],
                    qself: None,
                    path: ident.into(),
                })
            } else {
                syn::fold::fold_expr_macro(self, expr).into()
            }
        } else {
            syn::fold::fold_expr(self, expr)
        }
    }

    fn fold_expr_while(&mut self, expr: syn::ExprWhile) -> syn::ExprWhile {
        syn::ExprWhile {
            attrs: expr.attrs,
            label: expr.label,
            while_token: expr.while_token,
            cond: Box::new(if let syn::Expr::Let(cond) = *expr.cond {
                syn::Expr::Let(syn::ExprLet {
                    expr: Box::new(syn::Expr::Block(syn::ExprBlock {
                        attrs: vec![],
                        label: None,
                        block: self.fold_block(syn::Block {
                            brace_token: syn::token::Brace {
                                span: proc_macro2::Span::call_site(),
                            },
                            stmts: vec![syn::Stmt::Expr(*cond.expr)],
                        }),
                    })),
                    ..cond
                })
            } else {
                syn::Expr::Block(syn::ExprBlock {
                    attrs: vec![],
                    label: None,
                    block: self.fold_block(syn::Block {
                        brace_token: syn::token::Brace {
                            span: proc_macro2::Span::call_site(),
                        },
                        stmts: vec![syn::Stmt::Expr(*expr.cond)],
                    }),
                })
            }),
            body: self.fold_block(expr.body),
        }
    }
}

/// The main attribute, see crate level docs for details.
#[proc_macro_attribute]
pub fn ergo_pin(
    _attrs: proc_macro::TokenStream,
    code: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut visitor = Visitor::new();

    if let Ok(mac) = syn::parse::<syn::Macro>(code.clone()) {
        let tokens = mac.tokens;
        if let Ok(block) = syn::parse::<syn::Block>(quote!({ #tokens }).into()) {
            let block = visitor.fold_block(block);
            let tokens = block.stmts.into_iter().map(|stmt| quote!(#stmt)).collect();
            return syn::Macro { tokens, ..mac }.into_token_stream().into();
        }
    }

    if let Ok(item) = syn::parse::<syn::Item>(code.clone()) {
        return visitor.fold_item(item).into_token_stream().into();
    }

    if let Ok(block) = syn::parse::<syn::Block>(code.clone()) {
        return visitor.fold_block(block).into_token_stream().into();
    }

    panic!("Could not parse input")
}
