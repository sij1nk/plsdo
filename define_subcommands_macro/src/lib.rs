use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, parenthesized, parse_macro_input, Ident, LitStr, Result, Token};

struct Definition {
    name: Ident,
    description: LitStr,
}

impl Parse for Definition {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;

        let _paren_token = parenthesized!(content in input);
        let name = content.parse()?;
        let _comma_token: Token![,] = content.parse()?;
        let description = content.parse()?;

        Ok(Self { name, description })
    }
}

struct Input {
    definitions: Punctuated<Definition, Token![,]>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _bracket_token = bracketed!(content in input);

        Ok(Self {
            definitions: content.parse_terminated(Definition::parse)?,
        })
    }
}

#[proc_macro]
pub fn define_subcommands(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as Input);
    let definitions = input.definitions;

    let names = definitions.iter().map(|d| d.name.clone());
    let descriptions = definitions.iter().map(|d| d.description.clone());

    let expanded = quote! {
        const SUBCOMMANDS: &[(Definition, Script)] = &[
            #(
                (
                    (stringify!(#names), #descriptions, subcommands::#names::command_extension),
                    subcommands::#names::run
                ),
            )*
        ];
    };

    TokenStream::from(expanded)
}
