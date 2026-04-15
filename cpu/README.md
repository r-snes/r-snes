# R-SNES CPU software architecture documentation

This is a complete documentation of the relevant decisions made in this crate.

<div class="warning markdown-alert markdown-alert-warning"> <!-- we use selectors recognised by github AND rustdoc so it works in both -->
<!-- We have to use raw HTML links (and other elements) here because rustdoc doesn't parse markdown inside of HTML elements -->
If you’re just looking for information on how to use this crate, go to the documentation of the <a href="struct.CPU.html" title="struct cpu::cpu::CPU"><code>CPU</code></a> struct, or directly to the documentation of <a href="struct.CPU.html#method.cycle" title="method cpu::cpu::CPU::cycle"><code>CPU::cycle</code></a> (which is probably all you need to use the CPU in client code)
</div>

Below are explanations and justifications of the public API of the crate, the internal design of the `cycle` function, and the meta-language used to actually implement instructions. The particular implementations of instructions are not discussed however.

The end of this documentation compares this implementation of the WDC65C816 with implementations found in other emulators.

## Public API

The design of the current public API has been guided by the real world constraints which we learned along the way of trying to be emulate the CPU as accurately as possible:

To start with, since we want to have cycle-accurate emulation, we need to have a `cycle` function in the CPU which performs the actions in one cycle, no matter if it's an opcode fetch or an instruction cycle. This would require something along the lines of:
```rust
pub struct CPU {
    // private fields ...
}

impl CPU {
    pub fn cycle(&mut self, &mut impl Memory) {
        // ...
    }
}
```
(this public API implies that the CPU has arbitrary access the entire addressable space during each cycle, which isn't the case, but we'll come back to this later)

However, in the SNES, CPU cycles may take a different amount of time (counted in master cycles, which take a fixed amount of time) depending on what *type* of cycle the CPU performed. There are three types of cycles: read, write and internal.

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
Unless some read/write cycle timings also depend on the address at/from which we write/read. It's also a good time to mention that, actually, the CPU cannot access memory arbitrarily during a cycle: it can at read a single byte from memory, or write a single byte to memory (not both at the same time). This is not fundamentally a problem, but it means we would have to be very careful not to write or read more than a byte per cycle, and it would be annoying to properly test that we don't.

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
This way, client code can know where the CPU is accessing memory, and we directly encode the "one byte per cycle" constraint in our API in such a way that it is impossible to break the rule when implementing instruction.

## Internals

With the public API now defined, we need to find a decent way to implement it. This proves to be quite difficult actually, since the only method we have control over is the `cycle` function, and we have to figure out whether we need to fetch an opcode or if we're in the middle of an instruction, and if so where, and potentially in what branch (because some instruction may have some logical branches based on various conditions).

### Function pointer chains

At the start, we need one function to call per instruction (more precisely, per opcode), since after each opcode fetch we will have to jump the execution to the corresponding instruction.  
We can extend this idea of using function pointers.

The solution we ended up on gives the most possible control:

### Meta-language

## Comparison with other implementations
