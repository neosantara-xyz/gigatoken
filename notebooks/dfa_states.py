import marimo

__generated_with = "0.17.2"
app = marimo.App(width="full")


@app.cell
def _(CLASS_L, CLASS_N, CLASS_O, CLASS_Z):
    from __future__ import annotations
    from functools import lru_cache
    from dataclasses import dataclass
    from enum import IntEnum
    from unicodedata import category
    from collections import defaultdict
    from functools import lru_cache
    import marimo as mo

    class CLASS(IntEnum):
        """2 bit enum representing the character class of a code point"""
        L = 0
        N = 1
        Z = 2
        O = 3

        def __repr__(self):
            return self.name

        def as_bits(self):
            return tuple([int(b) for b in f'{self.value:02b}'])

    def cp_class(cp: int) -> int:
        if 0xD800 <= cp <= 0xDFFF:  # surrogates
            return CLASS.O
        c0 = category(chr(cp))[0]
        if c0 == 'L': return CLASS.L
        if c0 == 'N': return CLASS.N
        if c0 == 'Z': return CLASS.Z
        return CLASS.O


    def get_bits(b):
        return [int(i) for i in f"{b:08b}"]
    
    @dataclass(slots=True)
    class Codepoint:
        cp: int
        bytes: tuple[int]
        cls: CLASS

        @classmethod
        def from_utf32(cls, cp: int) -> Codepoint:
            bytes = tuple(chr(cp).encode('utf-8', 'surrogatepass'))
            clss = cp_class(cp)
            return cls(cp, bytes, clss)

        def as_sequence(self) -> tuple[int | CLASS]:
            return (*self.bytes, cls)

        @lru_cache(maxsize=None)
        def as_bit_sequence(self) -> tuple[int | CLASS, ...]:
            cls = self.cls
            return (*(bit for b in self.bytes for bit in get_bits(b)), cls)

        def __repr__(self):
            bytes_str = ' '.join(f'{b:02X}' for b in self.bytes)
            return f'{chr(self.cp)}[U+{self.cp:X}, {bytes_str}, {self.cls.name}]'

        def __str__(self):
            return repr(self)

        def __eq__(self, other):
            return self.cp == other.cp

        def __hash__(self):
            return hash(self.cp)


    def _codepoints():
        """Enumerate all Unicode code points"""
        res = []
        for cp in range(0x110000):
            res.append(Codepoint.from_utf32(cp))

        by_length: list[list[Codepoint]] = [[] for _ in range(5)]
        for cp in res:
            by_length[len(cp.bytes)].append(cp)

        return res, by_length
    codepoints, cp_by_length = _codepoints()

    def cp_class(cp: int) -> int:
        if 0xD800 <= cp <= 0xDFFF:  # surrogates
            return CLASS_O
        c0 = category(chr(cp))[0]
        if c0 == 'L': return CLASS_L
        if c0 == 'N': return CLASS_N
        if c0 == 'Z': return CLASS_Z
        return CLASS_O


    print(f'{len(codepoints)=}')
    return codepoints, defaultdict, mo


@app.cell(hide_code=True)
def _(mo):
    mo.md(
        r"""
    ## Figuring out DFA states
    After we've processed $i$ bits, we want to compress the processed information into as few states as possible.
    These states are found, by merging together all states that have the suffix.
    We represent a full codepoint entry as
    ```
    (bit_0, bit_1, ..., bit_(N-1), CLASS)
    ```
    This way, we can split it up to bit $i$, giving us a prefix and a suffix:

    ```
    prefix = (bit_0, bit_1, ..., bit_i)
    suffix = (bit_(i+1), ..., bit_(N-1), CLASS)
    ```
    We then merge all prefixes share the same set of suffixes into equivalence classes.
    This leads to the following relation

    Prefix $A$ leads to the same state as prefix $B \iff$ the set of suffixes that follow $A$ in the codepoint entry dataset is equal to the set of suffixes that follow $B$. 

    The code follows:
    """
    )
    return


@app.cell
def _(codepoints, defaultdict, mo):
    def general_merge(to_bits: int, codepoints=codepoints):
        # Group code points by their first n_bits
        by_first_section = defaultdict(list)
        for cp in codepoints:
            # seq = (bit0, bit1, bit2, ..., bitN, CLASS)
            # where CLASS in {0, 1, 2, 3}
            seq = cp.as_bit_sequence()
            prefix = seq[:to_bits]
            suffix = seq[to_bits:]
            by_first_section[prefix].append(suffix)

        # We want to merge all histories (prefixes) that have the same behavior (suffixes)
        merged = defaultdict(list)
        for first_section, s in by_first_section.items():
            merged[frozenset(s)].append(first_section)
        
        return len(merged)

    # 0         8          16          24          32
    # 11110uvv	10vvwwww	10xxxxyy	10yyzzzz
    for i in mo.status.progress_bar(range(0, 34, 1), show_eta=True, show_rate=True):
        n_states = general_merge(to_bits=i, codepoints=codepoints)
        print(f'You need {n_states:<3} states for the DFA after processing {i:<2} bits')
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(
        r"""
    The following overview shows much state we need after processing each bit:

    ```plaintext
    bitidx     0   1   2   3   4   5   6   7      8   9   10  11  12  13  14  15    16  17  18  19  20  21  22  23    24  25  26  27  28  29  30  31
    states     2   3   5   9   14  21  28  37     34  34  44  52  72  78  108 169   165 165 210 203 170 121 128 169   165 165 176 142 97  35  10  4   
    state bits 1   2   3   4   4   5   5   6      6   6   6   6   7   7   7   8     8   8   8   8   8   7   7   8     8   8   8   8   7   6   4   2
    ```

    The state bits are the state needed _after_ processing the bit at the given index.
    We see that we at most require 210 unique states when processing all 32 bits left to right.
    """
    )
    return


@app.cell
def _(mo):
    mo.md(
        r"""
    ### Using the DFA
    We can process the first 7 bits with a single [VPERMI2B](https://uops.info/table.html?search=vpermi2b&cb_lat=on&cb_tp=on&cb_uops=on&cb_ports=on&cb_ZEN4=on&cb_measurements=on&cb_doc=on&cb_base=on&cb_avx512=on), leading to 28 states, which fit within $\lceil\log_2(28)\rceil = 5$ bits.

    We then consume another 2 bits, (7 and 10, skipping bits 8 and 9, since these are always `1 0`), leading to 44 states = 6 state bits after processing 11 bits total.

    For bit 11, we do a VPERMI2B with the 6 state bits and 1 read bit, leading to 52 states.
    For bits 12 and 13, VPERMI2B twice, using the 6 state bits and 2 read bits to lookup to get a 78 element, 7-bit state.

    Here is an overview of the initial DFA process, showing the stage in which we process each bit, along with how many states result after processing those bits:
    ```plaintext
    bitidx     0                         8                         16                       24
    stage      1  1  1  1  1  1  1  2    -  -  2  3  4  4  x  x    -  -  x  x  x  x  x  x   -  -  x  x  x  x  x  x
    states     (       28        )  (    44    )  52 (78)
    state bits          5                6        6   7
    ```

    We are left with a 7-bit state, and 14 bits left to consume.
    Can split this into 14-bit lookup then 15-bit lookup (-2 bits with packing)?

    We can now continue using either a bunch more small VPERMI2B ops, or by using large, much more expensive [VPGATHERDD](https://uops.info/table.html?search=VPGATHERDD&cb_lat=on&cb_tp=on&cb_uops=on&cb_ports=on&cb_ZEN4=on&cb_measurements=on&cb_doc=on&cb_base=on&cb_avx512=on) ops to process many bits. 
    """
    )
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(
        r"""
    ## Notes
    Any time processing a single bit leads to a bit increase in the state size, we should process more bits at a time.
    """
    )
    return


if __name__ == "__main__":
    app.run()
