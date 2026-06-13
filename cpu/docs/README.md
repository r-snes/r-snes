# R-SNES CPU software architecture documentation

This is a complete documentation of the relevant design decisions made in this crate.

<div class="warning markdown-alert markdown-alert-warning"> <!-- we use selectors recognised by github AND rustdoc so it works in both -->
<!-- We have to use raw HTML links (and other elements) here because rustdoc doesn't parse markdown inside of HTML elements -->
If you’re just looking for information on how to use this crate, go to the documentation of the <a href="../cpu/struct.CPU.html" title="struct cpu::cpu::CPU"><code>CPU</code></a> struct, or directly to the documentation of <a href="../cpu/struct.CPU.html#method.cycle" title="method cpu::cpu::CPU::cycle"><code>CPU::cycle</code></a> (which is probably all you need to use the CPU in client code).
</div>

Below are explanations and justifications of the public API of the crate, the internal design of the `cycle` function, and the meta-language used to actually implement instructions. The particular implementations of instructions are not discussed however.

The end of this documentation compares this implementation of the WDC65C816 with implementations found in other emulators.

## Public API

The design of the current public API has been guided by the real world constraints which we learned along the way of trying to be emulate the CPU as accurately as possible:

To start with, since we want to have cycle-accurate emulation, we need to have a `cycle` function in the CPU which performs the actions of one cycle, no matter if it's an opcode fetch or an instruction cycle. This would require something along the lines of:
```rust
pub struct CPU {
    // private fields ...
}

impl CPU {
    pub fn cycle(&mut self, mem: &mut impl Memory) {
        // ...
    }
}
```
(this public API implies that the CPU has arbitrary access the entire addressable space during each cycle, which isn't the case, but we'll come back to this later)

However, in the SNES, CPU cycles may take a different amount of time (counted in master cycles, which take a fixed amount of time) depending on what type of cycle the CPU performed. There are three types of cycles: read, write and internal.

So we create this basic enum:
```rust
pub enum CycleResult {
    Read,
    Write,
    Internal,
}
```

And with that it seems natural to update our `cycle` function:
```rust
pub struct CPU {
    // private fields ...
}

impl CPU {
    pub fn cycle(&mut self, &mut impl Memory) -> CycleResult {
        // ...
    }
}
```

This way, we properly return the cycle type, which should allow the cycle timings to be correct, unless...
Unless some read/write cycle timings also depend on the address at/from which we write/read. It's also a good time to mention that, actually, the CPU cannot access memory arbitrarily during a cycle: it can either read a single byte from memory or write a single byte to memory (or neither in "internal" cycles), but never both at the same time. This is not fundamentally a problem, but it means we would have to be very careful not to write or read more than a byte per cycle, and it would be annoying to properly test that we don't.

To solve both of these problems, we end up on the following API:
```rust
pub struct CPU {
    pub data_bus: u8,

    // private fields ...
}

pub enum CycleResult {
    Read,
    Write,
    Internal,
}

impl CPU {
    pub fn cycle(&mut self) -> CycleResult {
        // ...
    }

    pub fn addr_bus(&self) -> &SnesAddress {
        &self.addr_bus
    }
}
```

The idea is that all data transfers go through the public `data_bus` member: when the cycle returns a `Write`, client code using the CPU writes the byte contained in the data bus at the address returned by `addr_bus`, whereas when the CPU returns a read cycle, the client code should write the value in the public `data_bus` field (this is why it is public).
This way, client code can know where the CPU is accessing memory, and we directly encode the "one byte per cycle" constraint in our API in such a way that it is impossible to break the rule when implementing instructions.

## Internals

With the public API now defined, we need to find a decent way to implement it. This proves to be quite difficult actually, since the only method we have control over is the `cycle` function, and we have to figure out whether we need to fetch an opcode or if we're in the middle of an instruction, and if so where, and potentially in what branch (because some instruction may have some logical branches based on various conditions).

### Function pointer chains

At the start, we need one function to call per instruction (more precisely, per opcode), since after each opcode fetch we will have to jump the execution to the corresponding instruction.
We can extend this idea of using function pointers.

The solution we ended up on gives the most possible control: we have a dedicated function for each possible cycle of each instruction. To describe what cycle is the next to be executed after a given cycle, each cycle function returns a function pointer to the next cycle function to execute. (along with the cycle type, which is strictly necessary to be able to implement the `cycle` public function correctly).

This results in the following code for the `cycle` function and related things:

```rust
// helper type required to write the recursive function signature
pub(crate) struct InstrCycle(pub fn(&mut CPU) -> (CycleResult, InstrCycle));

pub(crate) fn opcode_fetch(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
    cpu.addr_bus = SnesAddress {
        bank: cpu.registers.PB,
        addr: cpu.registers.PC,
    };

    (
        CycleResult::Read,
        InstrCycle(|next_cyc_cpu| (INSTR_CYC1[next_cyc_cpu.data_bus as usize].0)(next_cyc_cpu)),
    )
}

const INSTR_CYC1: [InstrCycle; 256] = [
    /* 00 */ InstrCycle(brk_cyc1),
    /* 01 */ InstrCycle(ora::dxind_cyc1),
    /* 02 */ InstrCycle(cop_cyc1),

    // ...

    /* fd */ InstrCycle(sbc::absx_cyc1),
    /* fe */ InstrCycle(inc_absx_cyc1),
    /* ff */ InstrCycle(sbc::abslx_cyc1),
];

pub struct CPU {
    pub data_bus: u8,

    pub(crate) next_cycle: InstrCycle,

    // other private fields...
}

impl CPU {
    pub fn cycle(&mut self) -> CycleResult {
        let (ret, next_cycle) = (self.next_cycle.0)(self);

        self.next_cycle = next_cycle;
        ret
    }
}
```

By returning arbitrary function pointers, we are able to account for whatever hardware quirk we may discover we need to implement, of which we found many. The two main categories are:

- Some instructions have "conditional idles" (branch instructions for example), which means they will optionally spend an internal cycle more than usual depending on some condition (for branch instructions: when the condition to branch out is met for example).
- All instructions which read or write the A, X or Y registers to memory will have 1 less cycle when the register (A, X, or Y) is in 8-bit mode compared to when it is in 16-bit mode, because we can only transfer one byte of the register per cycle.

### Meta-language

#### Choosing a way to implement the internal API

Now that we have defined the public API and a decent internal architecture to implement it, we still have to write all the cycle functions. Unfortunately, writing them by hand would produce a lot of code duplication that would be very hard to get rid of, since many behaviours we would like to extract so that they can be used for multiple instructions span more than one cycle, however we need to produce a function (more precisely a function pointer) per cycle, so we can't simply _extract these behaviours into functions_, because then the evaluation of these "helper" functions would not be able to produce _several functions_ (one for each cycle of the extracted behaviour).

Even with code duplication concerns aside, the implementation of each instruction cycle by cycle, with one function per cycle would be very hard to read and understand. For example, to implement by hand something as simple as an immediate LDA (loads an operand right after the opcode into the A register, and set some CPU flags depending on the loaded value), we would need this much code:

```rust
pub(crate) fn lda_imm_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
    if !cpu.registers.E && !cpu.registers.P.M {
        self::lda_imm_16_cyc1(cpu)
    } else {
        self::lda_imm_8_cyc1(cpu)
    }
}

pub(crate) fn lda_imm_8_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1u16);
    cpu.registers.PC = cpu.registers.PC.wrapping_add(2u16);
    (
        Read,
        InstrCycle(|cpu| {
            *cpu.registers.A.lo_mut() = cpu.data_bus;
            cpu.registers.P.Z = *cpu.registers.A.lo() == 0;
            cpu.registers.P.N = *cpu.registers.A.lo() > 0x7f;
            opcode_fetch(cpu)
        }),
    )
}

pub(crate) fn lda_imm_16_cyc1(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1u16);
    (Read, InstrCycle(lda_imm_cyc2))
}
pub(crate) fn lda_imm_16_cyc2(cpu: &mut CPU) -> (CycleResult, InstrCycle) {
    *cpu.registers.A.lo_mut() = cpu.data_bus;
    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(1);
    cpu.registers.PC = cpu.registers.PC.wrapping_add(3u16);
    (
        Read,
        InstrCycle(|cpu| {
            *cpu.registers.A.hi_mut() = cpu.data_bus;
            cpu.registers.P.Z = cpu.registers.A == 0;
            cpu.registers.P.N = cpu.registers.A > 0x7fff;
            opcode_fetch(cpu)
        }),
    )
}
```

As you can see, this also requires branching on the current size of the A register, and passing a new custom function (here a closure) for the "after the last" cycle of the instruction to inject some code to run during the next opcode fetch cycle, to be able to process the data we received after the `Read` request we made at the end of the actual last cycle completed.

For this kind of problems, we often need some form of meta-programming to generate (at compile time) code that is annoying to write but conceptually rather simple to understand (in comparison, at least).<br>
It might not be a surprise that once again, we ended up choosing the "most flexible" design possible, for it to handle any weird quirks we may encounter along the way.

An example will make more sense than just explaining things from zero:

```rust
cpu_instr!(lda_imm {
    meta SET_OP_SIZE AccMem;

    meta SET_ADDRMODE_IMM;
    meta FETCH_OP_INTO cpu.registers.A;
    meta SET_NZ_OP cpu.registers.A;
});
```

In a vacuum this seems like a whole lot of magic, but it actually concisely describes all an immediate LDA needs to do:

- `meta SET_OP_SIZE AccMem` set the "variable width" operations to depend on the size of A (which is the case for instructions touching A (the accumulator) or memory, hence the name `AccMem`). This basically "tells" the code generator that in the case it needs to create logical branches based on the size of a variable-width operand, it should use the condition `!cpu.registers.E && !cpu.registers.P.M` (`true` for the 16-bit case).
- `meta SET_ADDRMODE_IMM` properly sets the address bus to point to an "immediate" operand (an operand located right after the opcode of the instruction).
- `meta FETCH_OP_INTO cpu.registers.A` fetches a variable-width operand (which we previously set to depend on `AccMem`) into the `A` register. Quite notably, this will implicitly (automatically) generate "cycle boundaries", properly requesting read cycles, and then assigning the read value in its destination at the start of the next cycle.
- `meta SET_NZ_OP cpu.registers.A` sets the `N` and `Z` registers depending on the size of a variable-width operand (using the variable width we set to `AccMem`), `cpu.registers.A` in this case. This is needed especially for the `N` flag, because the comparison isn't the same between a `u8` and a `u16`: `u8` needs `N = op > 0x7f`, `u16` needs `N = op > 0x7fff`.

Actually, the above full written-out implementation of LDA imm with separate functions for each cycle is just the result of `cargo expand` on the macro call above (slight stripped down for readability, hiding some implementation details of the generating code).

This "meta-language" (effectively a DSL) started quite small and slowly grew to accomodate for more and more instructions to implement, and specific requirements they had. Documenting it completely would be tedious, and would likely lead to hard to keep up-to-date docs, but we can at least outline the main design decisions made.

#### Basic design of the meta language

In concept, the meta language is simple, or at least it tries to be. The main struct we are working with is this `InstrBody`: the body of an instruction, which is built by parsing the contents of the curly braces in the macro invocation (`cpu_instr!(instr_name { /* instr body */ });`).

Initially, this struct was simply this:

```rust
/// Body of an instruction: one or more cycles and potential
/// post-instruction code
#[derive(Default, Clone)]
pub(crate) struct InstrBody {
    /// Cycles of the instruction (does not include the opcode fetch cycle)
    pub cycles: Vec<Cycle>,
}

/// Data structure which contains the info required to build
/// a cycle function body
#[derive(Clone)]
pub(crate) struct Cycle {
    /// Raw function body
    pub body: TokenStream,

    /// Cycle type (part of the function return value; should evaluate
    /// to something of type `CycleResult`)
    pub cyc_type: Ident,
}
```

This is really simple: an instruction body is a list of cycles, which all have a fixed cycle type, and a given function body (the rust code which will be the body of the cycle function).

Then an abstraction we need quite early on are meta-instructions: the basic building block of the meta language. In the language itself, meta-instructions are things which start with the `meta` keyword, then any number of tokens until a semicolon.

It's about time we describe the basic logic of the metalanguage parser (some details are hidden, they will be explained later on):

```rust
impl InstrBody {
    pub fn parse(body: TokenStream) -> Result<Self, &'static str> {
        let mut it = body.into_iter().peekable();
        let mut ret = Self::default();

        loop {
            let it = it.by_ref();

            ret += it.take_while(|token| token.to_string() != "meta").collect::<TokenStream>();

            if it.peek().is_none() {
                break;
            }

            let meta_instr = MetaInstruction::try_from(it.take_while(|token| {
                let TokenTree::Punct(p) = token else {
                    return true;
                };
                return p.as_char() != ';';
            }))?;

            ret += meta_instr.expand();
        }
        Ok(ret)
    }
}
```

The logic is rather simple, so that it's simple to implement and easily predictable. We start with an empty `InstrBody` (using the `default()` constructor), then in a loop, we (in order):

- Append to the `InstrBody` (in the current cycle we are building up) any tokens found until we encounter a `meta` keyword (which denotes the start of a meta instruction). This is here to allow us to insert any rust code between meta instructions, and it will appear verbatim in the cycle function.
- (break out of the loop now if there are no more tokens, which will happen if some `InstrBody` ends with raw rust code instead of a meta instruction.)
- Parse the meta-instruction we found (between the `meta` keyword and the next semicolon, and then `expand` it into the `InstrBody` we're building up.

Meta instr expansions can contain both cycle code (code that will end up in one of the cycle functions) and cycle boundaries. This means that, if at a given point in parsing an `InstrBody`, we have two completed cycles and some code to be placed at the beginning of the third, a meta instr expansion can, for example:

- Just append code in the third cycle<br>
  → This is what some `SET_ADDRMODE_*` meta instrs do (immediate and stack for example, because they don't need cycles to know the address of the operand, so they just set a value in the address bus).
- Just "close" the third cycle as it is, assigning it a cycle type<br>
  → This is what the `END_CYCLE` meta-instruction does, closing the cycle with a cycle type given as parameter.
- Append code in the third cycle and close it with a given type<br>
  → This is what the `FETCH8_IMM` meta instr does: it places the address bus to point at an immediate operand, and marks the cycle as `Read`.
- Close the third cycle (giving it a type), and start writing code in the fourth cycle<br>
  → This is what the `FETCH8_IMM_INTO` meta instr does, it first generates the same thing as `FETCH8_IMM` (by calling it directly), and then generates an assignment from the read value to the destination passed as argument in the next cycle.
- Do the same for any number of cycles: for example append some code in the third, completely generate the fourth and fifth cycles, then start putting some code in the sixth.

This is the nice thing about meta instructions: they are composable, across cycles: we can use them to factor out logic that spans more than one cycle in a way that allows reuse without duplication.

#### How meta instructions compose

It is very easy to "compose" meta instructions (reuse the implementation of some to implement others), because conceptually, they expand to a 1-dimensional "stream" of `TokenStream`s (which can themselves be easily concatenated) interspersed with cycle boundaries, so it's conceptually easy to have a meta-instruction made up of `MetaInstrA::expand() + <some code> + MetaInstrB::expand()`.

In practice, meta instrs are expanded by this method:

```rust
    fn expand(self, pstate: &mut ParserState) -> MetaInstrExpansion {
        let mut ret = MetaInstrExpansion::default();

        match self {
            // cases...
        }
        ret
    }
```

The particular definition of `MetaInstrExpansion` will be explained later, but the key thing is that it implements `AddAssign` and `AddAssign<TokenStream>`, so that we can have for example:

```rust
    fn expand(self, pstate: &mut ParserState) -> MetaInstrExpansion {
        let mut ret = MetaInstrExpansion::default();

        match self {
            Self::SetAddrModeAbsLongX => {
                ret += Self::SetAddrModeAbsoluteLong.expand(pstate);
                ret += quote! {
                    cpu.addr_bus.addr = cpu.addr_bus.addr.wrapping_add(cpu.registers.X);
                }
            }

            // other cases...
        }
        ret
    }
```

In this example, `SetAddrModeAbsLongX` starts by using the expansion of `SetAddrModeAbsoluteLong`, and then adding the `X` increment on top of it.

#### Post-instruction code

We need "post-instruction" code for instructions which do _something_ with a value they read on their last cycle, because the read value only comes through on the next cycle (the one where we request a read for the opcode fetch).

To account for this, we need one more field in `InstrBody` which can contain an "unfinished" cycle, which gets us to our final definition of `InstrBody`:

```rust
/// Body of an instruction: one or more cycles and potential
/// post-instruction code
#[derive(Default, Clone)]
pub(crate) struct InstrBody {
    /// Cycles of the instruction (does not include the opcode fetch cycle)
    pub cycles: Vec<Cycle>,

    /// "Post-instruction" code: some code related to this instruction
    /// which needs to be run at the start of the next instruction.
    ///
    /// This is typically required when the last cycle of an
    /// instruction is a Read cycle, but the instruction needs to do
    /// something with the read value (e.g. placing it in a register).
    /// The problem is that the value will be injected between cycles, and
    /// will only be available at the start of the next cycle. So this code
    /// will be run at the beginning of the next opcode fetch cycle.
    pub post_instr: TokenStream,
}
```

Then, the code generator will take this `post_instr` into account when generating the last cycle function: if `post_instr` is empty, the next cycle function is simply `opcode_fetch`, but if it's not empty, we pass a closure as the next cycle function to execute, first running the `post_instr` code, and then jumping to the `opcode_fetch` directly:

```rust
if post_instr.is_empty() {
    format_ident!("opcode_fetch").into_token_stream()
} else {
    // inject the post-instr code in closure before returning to the opcode fetch.
    quote! {
        |cpu| {
            #post_instr
            opcode_fetch(cpu)
        }
    }
}
```

We could also generate a named function in place of a closure and pass that instead, either way works.

Also, the nice thing with this `InstrBody` type is that we're actually able to use it as the return type of `MetaInstruction::expand()`: meta instrs which just append code to the current cycle but don't create any new cycles can return an `InstrBody` with an empty `cycles` vector but some code in `post_instr`, and meta instrs which want to close a cycle without adding code in it can simply return a `cycles` vector containing a single `Cycle`, which has an empty body, so that when concatenating into another `InstrBody`, it will basically take its `post_instr`, and move it into a new `Cycle` in the `cycles` vector.<br>
This is what is done in this `AddAssign` impl:

```rust
impl std::ops::AddAssign for InstrBody {
    /// Concatenates an InstrBody to [`self`].
    /// [`self`] remains first after concatenation. [`self.post_instr`] will be
    /// prepended to the RHS's first cycle, or to the RHS's post_instr
    /// if it doesn't have any cycles.
    fn add_assign(&mut self, mut other: Self) {
        if other.cycles.is_empty() {
            // simple case: no cycle vec to merge, just join the post_instrs
            *self += other.post_instr;
            return;
        }

        // swap out self.post_instr with other.post_instr, such that
        // self.post_instr can be worked with later, and placing
        // other.post_instr already where it needs to be in the end
        let old_postinstr = std::mem::replace(&mut self.post_instr, other.post_instr);

        // swap out the first cycle body of the other InstrBody for our current
        // post_instr, and then glue it back *after* our current post_instr
        let second_firstcycle = std::mem::replace(other.cycles[0].body_mut(), old_postinstr);
        other.cycles[0].body_mut().extend(second_firstcycle);

        // finally merge the two cycle vectors
        self.cycles.extend(other.cycles);
    }
}
```

#### Automatic PC increment, address bus position tracking: `ParserState`

Something we found useful to implement some instructions is to have some state which the parser keeps across the expansion of meta instructions, and which they can access, like:

```rust
/// Data describing the state of the parser at any point in parsing
pub(crate) struct ParserState {
    // fields...
}

impl InstrBody {
    /// Parses a piece of instrbody, calling meta-instr expansions
    pub fn parse(body: TokenStream, pstate: &mut ParserState) -> Result<Self, &'static str> {
        // ...

        loop {
            // ...

            let meta_instr = (/* ... */);

            ret += meta_instr.expand(pstate);
        }
        Ok(ret)
    }
}
```

The important detail is that we pass `pstate` via a mutable reference to `meta_instr.expand()`, so meta instructions can affect the state of the parser, and potentially have some condition behaviour which depends on previous operations.

In practice, here is the actual `ParserState` (minus some fields which will be explained in the next section):

```rust
pub(crate) struct ParserState {
    /// Whether PC should be automatically incremented
    pub inc_pc: bool,

    /// Current position of the address bus. In some cases this can help
    /// avoiding some recalculations when switching from common addressing modes,
    /// for example: setting the addr mode to immediate at the start of an
    /// instr should only require increment the addr bus by 1 since it must
    /// point at the opcode (which is 1 before) initially.
    pub addrmode: AddrBusPosition,

    /// Offset from the PC to the next immediate operand.
    /// Each time an immediate byte is read, increment this so that independent
    /// calls to FetchImm can avoid reading the same byte multiple times.
    ///
    /// Also used to calculate the final PC increment to generate at the end
    /// of the instruction.
    pub imm_offset: u16,
}

#[derive(PartialEq, Eq)]
pub(crate) enum AddrBusPosition {
    /// The addr bus might point at any location in memory
    Unaligned,

    /// The addr bus points at the opcode of the current instruction
    Opcode,

    /// The addr bus points at the next immediate operand
    Immediate,
}
```

We use this to slightly optimise the generated code for the `SET_ADDRMODE_*` meta instrs, for example to not set the address bus to a position it is already set to, or to compute its value from a previous known value (i.e. for immediate reads) instead of computing it from the PC again.

In retrospect, it's likely that none of this is strictly necessary, but it's useful to have in development and could be used to track more "state" in the future.

#### Handling variable width operands

One of the (if not **the**) hardest quirks we had to work with to properly implement a cycle-accurate WDC65C816 is variable width operands. Registers A, X and Y can dynamically "change width" back and forth between 8- and 16-bit. This means that instructions which interact between these registers and memory will have to take one more cycle to complete when the register is 16 bits wide, because we can at most transfer 1 byte per cycle. This "variable width" also happens even in some instructions which don't access these registers, but act purely in memory, but still have their operand width (1 or 2 bytes) depend on the M flag in the CPU (which also controls the width of A). These are somehow even worse because they both read the value of their operand and then write it back to memory, so they take 2 more cycles in 16-bit version.

Even with cycle count concerns aside, there can also be some minor logic differences in the 8-bit and the 16-bit branch of these instructions, due to the variable width of the operand. For example, to set the N (negative) flag depending on the operand, we want to check if the operand would be a negative number if interpreted as signed, which requires different conditions depending on the size of the operand: for an 8-bit operand we check `op > 0x7f`, for a 16-bit one, we check `op > 0x7fff`.

To handle this, we created a `VarWidth` type:

```rust
/// Enum for all things which can have different values depending on the width
/// of a "variable width" instruction
pub(crate) enum VarWidth<T, U = ()> {
    /// Same value for 8-bit branch and the 16-bit branch
    ConstWidth(T),

    /// Potentially different values in the 8- and 16-bit branches
    /// of the instruction
    VarWidth{short: T, long: T, data: U},
}
```
(don't pay too much attention to the `data: U` yet, it's almost always `()` anyways)

Then, we change `InstrBody::parse()` function to return a `VarWidth<InstrBody>`, and meta instr expansions also are `VarWidth<InstrBody>`, so that we can have meta instrs that generate differente code in the 8-bit branch and the 16-bit branch.

We're able to allow easy composition of `VarWidth`s with the following `AddAssign` impls:

```rust
impl<T, U, V> AddAssign<V> for VarWidth<T, U>
where
    T: AddAssign<V>,
    V: Clone,
{
    fn add_assign(&mut self, x: V) {
        match *self {
            Self::ConstWidth(ref mut body) => *body += x,
            Self::VarWidth{ref mut short, ref mut long, ..} => {
                *short += x.clone();
                *long += x;
            }
        }
    }
}

impl<T1, U1, T2> AddAssign<VarWidth<T2, ()>> for VarWidth<T1, U1>
where
    T1: Clone + AddAssign<T2>,
    T2: Clone,
    U1: Default,
{
    fn add_assign(&mut self, other: VarWidth<T2, ()>) {
        match (self, other) {
            // simple case: other is constant width
            (self_, VarWidth::ConstWidth(x)) => *self_ += x,

            // both self and other are var width, add each respective part together
            (
                VarWidth::VarWidth{short: s_short, long: s_long, ..},
                VarWidth::VarWidth{short: o_short, long: o_long, ..},
            ) => {
                *s_short += o_short;
                *s_long += o_long;
            }

            // self is constant, other is variable: split self in half, then call the case above
            (self_@Self::ConstWidth(_), other@VarWidth::VarWidth{..}) => {
                let Self::ConstWidth(b) = self_ else { unreachable!(); };
                *self_ = Self::VarWidth{
                    short: b.clone(),
                    long: b.clone(),
                    data: U1::default(),
                };
                *self_ += other;
            }
        }
    }
}
```

With this, we now build incrementally a `VarWidth<InstrBody>` in `InstrBody::parse()` (intially constructed to be `ConstWidth` with 0 cycles) instead of a simple `InstrBody`, and while expanding meta instructions sequentially, as soon as a meta instr wants to generate width-dependent code, the current `InstrBody` is duplicated into a `VarWidth::VarWidth` with two identical bodies, then each branch of the meta instr expansion is concatenated in the appropriate branch of the (now split) `InstrBody`.<br>
This makes it so that we are building up two different versions of the instruction in parallel, as we expand meta instructions sequentially.

To register the condition which controls the variable width, we add a field in the `ParserState` and a meta instruction to change it:

```rust
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub(crate) enum OpSize {
    /// Constant operand size
    Constant,

    /// Operand size is either 8 or 16 bits, depending
    /// on the state of the M flag (width of the A register)
    AccMem,

    /// Operand size is either 8 or 16 bits, depending
    /// on the state of the X flag (width of X and Y registers)
    Index,
}

/// Data describing the state of the parser at any point in parsing
pub(crate) struct ParserState {
    /// Size of read/written operands of the instruction
    pub operand_size: OpSize,

    // other fields...
}

impl MetaInstruction {
    fn expand(self, pstate: &mut ParserState) -> MetaInstrExpansion {
        let mut ret = MetaInstrExpansion::default();

        match self {
            Self::SetOperandSize(arg) => {
                if pstate.operand_size != OpSize::Constant {
                    panic!("Operand size can only be set once!");
                }
                match arg.to_string().as_str() {
                    "AccMem" => pstate.operand_size = OpSize::AccMem,
                    "Index" => pstate.operand_size = OpSize::Index,
                    _ => panic!("Only valid operand sizes are AccMem and Index")
                }
            }

            // other cases ...
        }
        ret
    }
}
```

By default, we initialise `operand_size` to `Constant` in the `ParserState`, so that instructions are considered constant size by default.

And then to forward this condition up in the complete `Instr`, we change `body` to be a `VarWidth<InstrBody, TokenStream>`:

```rust
/// Type resulting from the parsing of the token stream passed
/// as parameter to the [`cpu_instr`] proc macro.
pub(crate) struct Instr {
    /// Name of the instruction (e.g. clv, inx, ldx_imm, jmp_abs)
    pub name: Ident,

    /// Body of the instruction, including potential post-instr code,
    /// and 16-/8-bit disjunction
    pub body: VarWidth<InstrBody, TokenStream>,
}

impl Instr {
    pub fn parse(stream: TokenStream, inc_pc: bool) -> Result<Self, &'static str> {
        let mut pstate = ParserState::default();
        pstate.inc_pc = inc_pc;

        let mut it = stream.into_iter();
        let Some(TokenTree::Ident(name)) = it.next() else {
            Err("Expecting the instruction name")?
        };
        let Some(TokenTree::Group(body)) = it.next() else {
            Err("Expecting the instruction body")?
        };
        if it.next().is_some() {
            Err("Unexpected token after the instruction body")?
        }

        let mut ret = Self::new(name);
        ret.body += InstrBody::parse(body.stream(), &mut pstate)?;

        // ...

        // here `data` is the condition in which the instr is in 16-bit mode
        if let VarWidth::VarWidth{ref mut data, .. } = ret.body {
            match pstate.operand_size {
                OpSize::Constant => panic!(
                    "Variable width instructions used without setting the operand size"
                ),
                OpSize::Index => *data = quote!(!cpu.registers.E && !cpu.registers.P.X),
                OpSize::AccMem => *data = quote!(!cpu.registers.E && !cpu.registers.P.M),
            }
        }
        Ok(ret)
    }
}
```

This is the only case where we use the `data` field of `VarWidth::VarWidth` to attach some payload data in the variable-width variant of the enum: in this case, the token stream encoding the variable width condition.

The last change we need to make is in `ParserState`'s `imm_offset`: some instructions (immediate loads in particular) will not consume the same amount of bytes of immediate operands depending on the instruction width, so we change it to be a `VarWidth` too:

```rust
/// Data describing the state of the parser at any point in parsing
pub(crate) struct ParserState {
    /// Whether PC should be automatically incremented
    pub inc_pc: bool,

    /// Current position of the address bus. In some cases this can help
    /// avoiding some recalculations when switching from common addressing modes,
    /// for example: setting the addr mode to immediate at the start of an
    /// instr should only require increment the addr bus by 1 since it must
    /// point at the opcode (which is 1 before) initially.
    pub addrmode: AddrBusPosition,

    /// Offset from the PC to the next immediate operand.
    /// Each time an immediate byte is read, increment this so that independent
    /// calls to FetchImm can avoid reading the same byte multiple times.
    ///
    /// Also used to calculate the final PC increment to generate at the end
    /// of the instruction.
    pub imm_offset: VarWidth<u16>,

    /// Size of read/written operands of the instruction
    pub operand_size: OpSize,
}
```

We take this variable-width PC offset correctly into account when generating the automatic PC increment at the end of the last cycle of the instruction.

## Comparison with other implementations

To be done

<!-- Small draft I started but failed to get actual examples to prove my point:

Many small-scale implementations (if not most) don't even try to achieve cycle accuracy, so if they provide a `cycle` function, it mostly looks like this:

```rust
impl CPU {
    fn cycle(&mut self, mem: &impl Memory) {
        self.cycles_remaining -= 1;

        if self.cycles_remaining != 0 {
            return;
        }

        self.cycles_remaining += self.run_instr(mem);
    }
}
```
(where `run_instr` would do an opcode fetch, then execute all cycles of the instruction in one go, and return the number of cycles to wait)
-->
