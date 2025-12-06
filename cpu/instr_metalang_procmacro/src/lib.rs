mod parser;

use parser::{Cycle, Instr, InstrBody, InstrCycles, VarWidthInstr};
use proc_macro2::{TokenStream, Ident};
use quote::{format_ident, quote, ToTokens};

fn gen_cycle_functions(name: &Ident, instr_body: InstrBody) -> TokenStream {
    let cycles = &instr_body.cycles;
    let post_instr = &instr_body.post_instr;

    cycles
        .iter()
        .enumerate()
        .map(|(i, cyc)| {
            let func_name = format_ident!("{}_cyc{}", name, i + 1);
            let next_func_name: TokenStream = if i != cycles.len() - 1 {
                format_ident!("{}_cyc{}", name, i + 2).into_token_stream()
            } else {
                if post_instr.is_empty() {
                    format_ident!("opcode_fetch").into_token_stream()
                } else {
                    // inject the post-instr code in closure before returning to
                    // the opcode fetch.
                    quote! {
                        |cpu| {
                            #post_instr
                            opcode_fetch(cpu)
                        }
                    }
                }
            };


            let (body, cyc_type) = match cyc {
                Cycle::Unconditional{body, cyc_type} => (body, cyc_type),
                Cycle::ConditionalIdle{body, condition} => (
                    &quote! {
                        #body
                        if !(#condition) {
                            return (#next_func_name)(cpu);
                        }
                    },
                    &quote!(Internal),
                ),
            };

            quote! {
                pub(crate) fn #func_name(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    #body

                    (#cyc_type, InstrCycle(#next_func_name))
                }
            }
        }).collect::<TokenStream>()
}

/// Function that actually implements all the logic for the proc macro,
/// using the types provided by the `proc_macro2` crate, which have the
/// advantage of also existing outside of proc macro crates; and therefore
/// have more utilities built around them, which makes unit-testing easier,
/// among many other things.
pub(crate) fn cpu_instr2(input: TokenStream, inc_pc: bool) -> TokenStream {
    let Instr { name, body } = match parser::Instr::parse(input, inc_pc) {
        Ok(instr) => instr,
        Err(msg) => panic!("{}", msg),
    };

    let cycle_funcs = match body {
        InstrCycles::ConstantWidth(instr_body) => gen_cycle_functions(&name, instr_body),
        InstrCycles::VariableWidth(VarWidthInstr {body_8bits, body_16bits, condition}) => {
            let cyc_funcs8 = gen_cycle_functions(&name, body_8bits);
            let cyc_funcs16 = gen_cycle_functions(&name, body_16bits);

            let first_cyc_name = format_ident!("{}_cyc1", name);

            quote! {
                pub(crate) fn #first_cyc_name(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                    if #condition {
                        self::_16::#first_cyc_name(cpu)
                    } else {
                        self::_8::#first_cyc_name(cpu)
                    }
                }

                pub(crate) mod _8 {
                    use crate::instrs::prelude::*;
                    #cyc_funcs8
                }
                pub(crate) mod _16 {
                    use crate::instrs::prelude::*;
                    #cyc_funcs16
                }
            }
        },
    };

    // wrap the generated instruction in a submodule for it to be easier
    // to expand using cargo_expand
    quote! {
        pub(crate) use #name::*;
        pub(crate) mod #name {
            use crate::instrs::prelude::*;

            #cycle_funcs
        }
    }
}

/// The main proc macro of this crate.
///
/// Syntax:
/// ```rs
/// cpu_instr!(instr_name {
///     instruction_body();
/// });
/// ```
///
/// This macro can be called at the top level (not necessarily within a
/// function body) since it will only produce function definitions.
///
/// The instruction name (`instr_name` in the above example) will be used
/// as the base for the function name(s) that will be generated, in the
/// following format: `("{}_cyc{}", instr_name, cycnum)`, where `cycnum`
/// is the cycle number starting at 1.
///
/// The instruction body comes in curly braces after the instruction name.
/// The body accepts special syntax (*meta instructions*) as well as
/// function-body Rust code.  
/// Meta-instructions are a statement starting by a `meta` identifier,
/// followed by a meta-instruction name (usually in all uppercase),
/// followed by operands and then a semicolon.
///
/// The most fundamental meta-instruction is `END_CYCLE`,
/// which taks a cycle type as parameter, and act as a cycle delimiter;
/// resulting in a separate function for each cycle.
///
/// For a reference of the available meta-instructions
/// and their behaviour, see the (not yet done because the language is still
/// very much subject to change) meta language reference in the module
/// documentation.
#[proc_macro]
pub fn cpu_instr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cpu_instr2(input.into(), true).into()
}

/// Same as `cpu_instr` but disables all PC increments that would
/// automatically be generated in the instruction code
///
/// This is especially useful for instructions which assign a value to PC,
/// (e.g. jumps and returns) because the automatically-generated PC increments
/// would conflict with what the instruction is trying to do to set the PC.
#[proc_macro]
pub fn cpu_instr_no_inc_pc(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cpu_instr2(input.into(), false).into()
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
        assert_tokstream_eq(cpu_instr2(macro_input, false), exp_output)
    }

    fn assert_macro_incpc_produces(macro_input: TokenStream, exp_output: TokenStream) {
        assert_tokstream_eq(cpu_instr2(macro_input, true), exp_output)
    }

    #[test]
    fn test_inx() {
        assert_macro_produces(
            quote!(instr_inx {
                cpu.registers.X = cpu.registers.X.wrapping_add(1);
                cpu.registers.P.Z = cpu.registers.X == 0;
                cpu.registers.P.N = cpu.registers.X > 0x7fff;

                meta END_CYCLE Internal;
            }),
            quote!(
                pub(crate) use instr_inx::*;
                pub(crate) mod instr_inx {
                    use crate::instrs::prelude::*;

                    pub(crate) fn instr_inx_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        cpu.registers.X = cpu.registers.X.wrapping_add(1);
                        cpu.registers.P.Z = cpu.registers.X == 0;
                        cpu.registers.P.N = cpu.registers.X > 0x7fff;

                        (Internal, InstrCycle(opcode_fetch))
                    }
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
                meta END_CYCLE Write;
            }),
            quote!(
                pub(crate) use some_instr::*;
                pub(crate) mod some_instr {
                    use crate::instrs::prelude::*;

                    pub(crate) fn some_instr_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        some_function1(cpu);

                        (Internal, InstrCycle(some_instr_cyc2))
                    }
                    pub(crate) fn some_instr_cyc2(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        some_function2(cpu);

                        (Read, InstrCycle(some_instr_cyc3))
                    }
                    pub(crate) fn some_instr_cyc3(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        some_function3(cpu);

                        (Write, InstrCycle(opcode_fetch))
                    }
                }
            ),
        );
    }

    #[test]
    fn conditional_cycle_type() {
        assert_macro_produces(
            quote!(test_instr {
                meta END_CYCLE if 1 == 0 { Internal } else { Read };

                meta END_CYCLE some_func_which_determines_cyc_type();
            }),
            quote!(
                pub(crate) use test_instr::*;
                pub(crate) mod test_instr {
                    use crate::instrs::prelude::*;

                    pub(crate) fn test_instr_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        (if 1 == 0 { Internal } else { Read }, InstrCycle(test_instr_cyc2))
                    }

                    pub(crate) fn test_instr_cyc2(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        (some_func_which_determines_cyc_type(), InstrCycle(opcode_fetch))
                    }
                }
            ),
        );
    }

    #[test]
    fn post_cycle_code() {
        assert_macro_produces(
            quote!(test_instr {
                meta END_CYCLE Read;

                cpu.registers.X = cpu.data_bus as u16;
            }),
            quote!(
                pub(crate) use test_instr::*;
                pub(crate) mod test_instr {
                    use crate::instrs::prelude::*;

                    pub(crate) fn test_instr_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        (Read, InstrCycle(|cpu| {
                            cpu.registers.X = cpu.data_bus as u16;

                            opcode_fetch(cpu)
                        }))
                    }
                }
            ),
        );
    }

    #[test]
    fn auto_inc_pc() {
        assert_macro_incpc_produces(
            quote!(test_instr {
                call_func1();
                meta END_CYCLE Internal;

                call_func2();
                meta END_CYCLE Internal;
            }),
            quote!(
                pub(crate) use test_instr::*;
                pub(crate) mod test_instr {
                    use crate::instrs::prelude::*;

                    pub(crate) fn test_instr_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        call_func1();

                        (Internal, InstrCycle(test_instr_cyc2))
                    }
                    pub(crate) fn test_instr_cyc2(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
                        call_func2();

                        cpu.registers.PC = cpu.registers.PC.wrapping_add(1u16);
                        (Internal, InstrCycle(opcode_fetch))
                    }
                }
            ),
        );
    }
}
