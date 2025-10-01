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

    fn assert_tokstream_eq(actual: TokenStream, expected: TokenStream) {
        let Ok(actual) = syn::parse2::<syn::File>(actual) else {
            panic!("assert_tokstream_eq: actual doesn't parse");
        };
        let Ok(expected) = syn::parse2::<syn::File>(expected) else {
            panic!("assert_tokstream_eq: expected doesn't parse");
        };

        assert_eq!(
            actual,
            expected,
            "\n=====\nActual:\n{}\n=====\nExpected:\n{}\n",
            prettyplease::unparse(&actual),
            prettyplease::unparse(&expected)
        )
    }

    fn assert_macro_produces(macro_input: TokenStream, exp_output: TokenStream) {
        assert_tokstream_eq(cpu_instr2(macro_input), exp_output)
    }

    #[test]
    fn test_inx() {
        assert_macro_produces(
            quote!(instr_inx {
                cpu.registers.X = cpu.registers.X.wrapping_add(1);
                cpu.registers.P.Z = cpu.registers.X == 0;
                cpu.registers.P.N = cpu.registers.X > 0x7fff;

                meta END_INSTR Internal;
            }),
            quote!(
                pub(crate) fn instr_inx_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    cpu.registers.X = cpu.registers.X.wrapping_add(1);
                    cpu.registers.P.Z = cpu.registers.X == 0;
                    cpu.registers.P.N = cpu.registers.X > 0x7fff;

                    (CycleResult::Internal, InstrCycle(opcode_fetch))
                }
            ),
        );
    }

    #[test]
    fn test_some_instr() {
        assert_macro_produces(
            quote!(some_instr {
                some_function1(cpu);
                meta END_CYCLE Internal;

                some_function2(cpu);
                meta END_CYCLE Read;

                some_function3(cpu);
                meta END_INSTR Write;
            }),
            quote!(
                pub(crate) fn some_instr_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    some_function1(cpu);

                    (CycleResult::Internal, InstrCycle(some_instr_cyc2))
                }
                pub(crate) fn some_instr_cyc2(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    some_function2(cpu);

                    (CycleResult::Read, InstrCycle(some_instr_cyc3))
                }
                pub(crate) fn some_instr_cyc3(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    some_function3(cpu);

                    (CycleResult::Write, InstrCycle(opcode_fetch))
                }
            ),
        );
    }
}
