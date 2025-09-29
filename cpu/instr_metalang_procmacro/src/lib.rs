mod parser;

use parser::{Cycle, Instr};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(crate) fn cpu_instr2(input: TokenStream) -> TokenStream {
    let Instr { name, cycles } = match parser::Instr::try_from(input) {
        Ok(instr) => instr,
        Err(msg) => panic!("{}", msg),
    };

    let cycle_funcs = cycles
        .iter()
        .enumerate()
        .map(|(i, Cycle { body, cyc_type })| {
            let func_name = format_ident!("{}_cyc{}", name, i + 1);
            let next_func_name = if i != cycles.len() - 1 {
                format_ident!("{}_cyc{}", name, i + 2)
            } else {
                format_ident!("opcode_fetch")
            };

            quote! {
                pub(crate) fn #func_name(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    #body

                    (CycleResult::#cyc_type, InstrCycle(#next_func_name))
                }
            }
        });

    TokenStream::from_iter(cycle_funcs)
}

#[proc_macro]
pub fn cpu_instr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cpu_instr2(input.into()).into()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_inx() {
        let input = quote!(instr_inx {
            cpu.registers.X = cpu.registers.X.wrapping_add(1);
            cpu.registers.P.Z = cpu.registers.X == 0;
            cpu.registers.P.N = cpu.registers.X > 0x7fff;

            meta END_INSTR Internal;
        });

        let output = cpu_instr2(input);

        print!("{}", prettyplease::unparse(&syn::parse2(output).unwrap()))
    }

    #[test]
    fn test_some_instr() {
        let input = quote!(some_instr {
            some_function1(cpu);
            meta END_CYCLE Internal;

            some_function2(cpu);
            meta END_CYCLE Read;

            some_function3(cpu);
            meta END_INSTR Write;
        });

        let output = cpu_instr2(input);

        print!("{}", prettyplease::unparse(&syn::parse2(output).unwrap()))
    }
}
