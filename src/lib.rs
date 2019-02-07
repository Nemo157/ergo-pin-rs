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
            if expr.mac.path.is_ident(pin) {
                let ident = self.gen_ident();
                self.pinned
                    .push((ident.clone(), syn::parse(expr.mac.tts.into()).unwrap()));
                return syn::ExprPath {
                    attrs: vec![],
                    qself: None,
                    path: ident.into(),
                }
                .into();
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
            cond: Box::new(
                if let syn::Expr::Let(cond) = *expr.cond {
                    syn::ExprLet {
                        attrs: cond.attrs,
                        let_token: cond.let_token,
                        pats: cond.pats,
                        eq_token: cond.eq_token,
                        expr: Box::new(syn::ExprBlock {
                            attrs: vec![],
                            label: None,
                            block: self.fold_block(syn::Block {
                                brace_token: syn::token::Brace {
                                    span: proc_macro2::Span::call_site(),
                                },
                                stmts: vec![syn::Stmt::Expr(*cond.expr)],
                            }),
                        }.into()),
                    }.into()
                } else {
                    syn::ExprBlock {
                        attrs: vec![],
                        label: None,
                        block: self.fold_block(syn::Block {
                            brace_token: syn::token::Brace {
                                span: proc_macro2::Span::call_site(),
                            },
                            stmts: vec![syn::Stmt::Expr(*expr.cond)],
                        }),
                    }.into()
                }
            ),
            body: self.fold_block(expr.body),
        }
    }
}

/// # Examples
///
/// ```rust
/// #![feature(generators, generator_trait)]
///
/// use core::ops::{Generator, GeneratorState};
/// use ergo_pin::ergo_pin;
///
/// #[ergo_pin]
/// fn foo() -> GeneratorState<usize, ()> {
///     pin!(static || { yield 5 }).resume()
/// }
///
/// assert_eq!(foo(), GeneratorState::Yielded(5));
/// ```
///
/// ```rust
/// #![feature(generators, generator_trait, stmt_expr_attributes, proc_macro_hygiene)]
///
/// use core::ops::{Generator, GeneratorState};
/// use ergo_pin::ergo_pin;
///
/// fn foo() -> GeneratorState<usize, ()> {
///     #[ergo_pin] {
///         pin!(static || { yield 5 }).resume()
///     }
/// }
///
/// assert_eq!(foo(), GeneratorState::Yielded(5));
/// ```
///
/// ```rust
/// #![feature(generators, generator_trait, proc_macro_hygiene)]
///
/// use core::ops::{Generator, GeneratorState};
/// use ergo_pin::ergo_pin;
///
/// macro_rules! bar {
///     ($($tts:tt)+) => { $($tts)+ };
/// }
///
/// fn foo() -> GeneratorState<usize, ()> {
///     #[ergo_pin]
///     bar! {
///         pin!(static || { yield 5 }).resume()
///     }
/// }
///
/// assert_eq!(foo(), GeneratorState::Yielded(5));
/// ```
#[proc_macro_attribute]
pub fn ergo_pin(
    _attrs: proc_macro::TokenStream,
    code: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if let Ok(fun) = syn::parse::<syn::ItemFn>(code.clone()) {
        return Visitor::new().fold_item_fn(fun).into_token_stream().into();
    }

    if let Ok(block) = syn::parse::<syn::Block>(code.clone()) {
        return Visitor::new().fold_block(block).into_token_stream().into();
    }

    if let Ok(syn::Macro {
        path,
        bang_token,
        delimiter,
        tts,
    }) = syn::parse::<syn::Macro>(code.clone())
    {
        if let Ok(block) = syn::parse::<syn::Block>(quote!({ #tts }).into()) {
            let block = Visitor::new().fold_block(block);
            let tts = block.stmts.into_iter().map(|stmt| quote!(#stmt)).collect();
            let mac = syn::Macro {
                path,
                bang_token,
                delimiter,
                tts,
            };
            return mac.into_token_stream().into();
        }
    }

    panic!("Could not parse input")
}
