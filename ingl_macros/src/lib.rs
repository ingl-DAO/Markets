extern crate proc_macro;

extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};

#[proc_macro_derive(Validate, attributes(validation_phrase))]
pub fn validate_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_validate(&ast)
}

#[proc_macro_derive(InglErr, attributes(err))]
pub fn ingl_err_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_ingl_err(&ast)
}

fn impl_ingl_err(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    match &ast.data {
        syn::Data::Enum(data) => {
            let mut variants = Vec::new();
            let mut tests = Vec::new();
            for variant in &data.variants {
                match variant
                    .attrs
                    .iter()
                    .position(|attr| attr.path.is_ident("err"))
                {
                    Some(index) => {
                        let error_message = variant.attrs[index].tokens.clone();
                        let variant_name = &variant.ident;
                        let variant_name_str = variant_name.to_string();
                        let test_ident = &Ident::new(
                            &(variant_name_str.to_owned() + "_error_test"),
                            Span::call_site(),
                        );
                        variants.push(
                            quote!{
                                #name::#variant_name => {
                                    colored_log!(LOG_LEVEL, 5, Red, "Error: {:?}. Keyphrase = {:?}",#error_message, keyword)
                                }
                            }
                        );

                        tests.push(
                            quote!{
                                #[test]
                                fn #test_ident(){
                                    assert_eq!(#name::#variant_name.utilize(""), ProgramError::Custom(#name::#variant_name as u32));
                                }

                            }
                        );
                    }
                    None => panic!("All variants must have an 'err' attribute"),
                }
            }

            let gen = quote! {
                impl #name{
                    pub fn utilize(self, keyword: &str) -> ProgramError{
                        match self{
                            #( #variants )*
                        }

                        ProgramError::Custom(self as u32)
                    }

                }


                #[cfg(test)]
                mod ingl_err_tests{
                    use super::*;
                    use solana_program::program_error::ProgramError;
                    use solana_program::msg;

                    #( #tests )*
                }
            };
            gen.into()
        }
        _ => panic!("InglErr can only be derived for Enums"),
    }
}

fn impl_validate(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let index = match ast.attrs.iter().position(|attr| attr.path.is_ident("validation_phrase")){
        Some(index) => index,
        None => panic!("Struct must have a 'validation_phrase' attribute. use #[validation_phrase = 987_654_321u32]"),
    };
    let validation_phrase = &(ast.attrs[index].tokens);
    let name_str = name.to_string();
    let mod_ident = &Ident::new(
        &(name_str.to_owned() + "_validation_test"),
        Span::call_site(),
    );

    let gen = quote! {
        impl #name {
            ///Verifies that the validation_phrase in the struct is similar to the validation_phrase in the attribute
            pub fn validate(self) -> Result<Self, ProgramError> {
                // msg!("sent phrase {:?}, expected {:?}",self.validation_phrase, #validation_phrase);
                match (self.validation_phrase == #validation_phrase){
                    true => Ok(self),
                    false => Err(InglError::InvalidValPhrase.utilize(#name_str)),
                }
            }
            ///converts the data in an accountInfo of apporiate type into the inherited struct.
            /// validates the data using the self.validate() method
            /// asserts that the accountInfo is owned by the program
            pub fn decode_unchecked(account: &AccountInfo) -> Result<Self, ProgramError> {
                let a: Self = try_from_slice_unchecked(&account.data.borrow()).error_log(format!("Error while decoding using try_from_slice_unchecked: {:?}.", #name_str).as_str())?;
                a.validate().error_log(format!("Error while validating: {:?}.", #name_str).as_str())
            }
            ///converts the data in an accountInfo of apporiate type into the inherited struct.
            /// validates the data using the self.validate() method
            /// asserts that the accountInfo is owned by the program_id sent
            pub fn parse(account: &AccountInfo, program_id: &Pubkey) -> Result<Self, ProgramError> {
                match account.assert_owner(program_id){
                    Ok(_) => {
                        let a: Self = try_from_slice_unchecked(&account.data.borrow()).error_log(format!("Error while decoding using try_from_slice_unchecked: {:?}.", #name_str).as_str())?;
                        a.validate().error_log(format!("Error while validating: {:?}.", #name_str).as_str())
                    },
                    Err(e) => {
                        colored_log!(LOG_LEVEL, 5, Red, "Error while decoding: {:?}", #name_str);
                        Err(e)},
                }
            }


        }

        #[cfg(test)]
        mod #mod_ident{
            use std::any::type_name;
            fn type_of<T>(_: T) -> &'static str {
                type_name::<T>()
            }
            #[test]
            fn is_valid_validation_phrase() {
                assert_eq!(type_of(#validation_phrase), "u32");
            }
        }
    };
    gen.into()
}
