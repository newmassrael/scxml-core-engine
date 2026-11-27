// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The one read of a `<sce:tlv-chain>` into its list, and the one scan of a
//! chain a view keeps as its bytes (docs/adr/0136).
//!
//! A chain is read by a loop: read an entry, read off it where the chain ends
//! and what the next entry is handed, note the kinds of entry the gates ask
//! for, hold the entry where the list has room, and judge how the chain ended.
//! Written into each codec that holds a chain, that loop was a copy for every
//! such codec, printed in full. Here it is one function, out of line, and what
//! differs from chain to chain is the rule of the chain: how an entry is read
//! with the inputs the chain hands it, where the chain ends, what it carries
//! from one entry to the next, and which kinds of entry the gates on it ask
//! for. The rule is a type, a marker the generated codec declares for each
//! chain ([`ChainRule`], [`ChainScanRule`]), and each part of it a function or
//! a constant of that type: the routine is instantiated at the rule and calls
//! each part by name. No part is a function pointer, so a bare-metal image keeps
//! a call graph every call of which has its callee, and its stack bound is one
//! a tool can prove.
//!
//! Both read through the frame's reader (docs/adr/0128): every read is total
//! and the first refusal sticks, so the loop tests the failure at its head and
//! nowhere else, and records every refusal of the chain in the reader.

use crate::codec::{push_keeping, CodecError, EntryKinds};
use crate::reader::SceReader;
use crate::records::{show_entry, ChainHook, ChainSite};
use crate::storage::HeldList;
use crate::view::ListWalk;

/// How a chain ends (`sce:terminate-on`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainEnd {
    /// With its frame, or at its depth: an entry is read while bytes remain.
    Frame,
    /// After the first entry whose flag, which the rule reads off the entry
    /// ([`ChainRule::follows`], [`ChainScanRule::follows`]), is clear. Such a
    /// chain is one entry or more (docs/adr/0065 §6 R7), so its first entry is
    /// read whatever follows.
    Flag,
}

impl ChainEnd {
    /// Whether the chain ends with its frame, so that no entry is read once no
    /// byte remains.
    pub(crate) const fn with_frame(self) -> bool {
        matches!(self, ChainEnd::Frame)
    }
}

/// Where one chain ends, whatever form of its entry it is read as: what the
/// eager read of the chain ([`ChainRule`]) and a view's scan of it
/// ([`ChainScanRule`]) share. A generated codec implements it on the marker
/// type it declares for the chain.
pub trait ChainEnds {
    /// How the chain ends.
    const END: ChainEnd;
    /// The most entries the chain holds, `None` where the protocol states no
    /// depth (docs/adr/0077 §7).
    const DEPTH: Option<u32>;
}

/// How one chain of an eager decode is read ([`read_chain_into`]): the rule of
/// the chain whose entries are `E`, each read with `K` inputs, from a frame of
/// the lifetime `'f`. A generated codec implements it on the marker type it
/// declares for the chain, beside [`ChainEnds`].
pub trait ChainRule<'f, E, const K: usize>: ChainEnds {
    /// The kinds of entry the gates on the chain ask for (docs/adr/0065 §6 R6):
    /// [`NoKinds`](crate::codec::NoKinds) for a chain no gate reads.
    type Kinds: EntryKinds<E>;

    /// Read one entry from `reader`, handed the inputs `args` in the order the
    /// entry declares them: what the parent hands every entry, and what the
    /// chain carried from the entry before.
    fn read(reader: &mut SceReader<'f>, args: &[u8; K]) -> E;

    /// Whether `entry` declares an entry after it: the flag a chain that ends
    /// on a flag ([`ChainEnd::Flag`]) ends where it is clear. A chain that ends
    /// with its frame asks it of no entry, and keeps this answer.
    fn follows(_entry: &E) -> bool {
        false
    }

    /// Rewrite the values the chain carries from `entry`, the entry just read
    /// (`<sce:entry-flag-bind>`), in `args`. A chain that carries nothing
    /// keeps this, which rewrites none.
    fn carry(_entry: &E, _args: &mut [u8; K]) {}
}

/// How a view's scan reads one chain beside its walk ([`walk_chain`]): where
/// the chain ends, read off the entry as the walk reads it, `E`. The walk
/// reads each entry and carries what the chain carries ([`ListWalk`]); a
/// generated codec implements this on the marker type of the chain, beside
/// [`ChainEnds`].
pub trait ChainScanRule<E>: ChainEnds {
    /// Whether `entry` declares an entry after it, as [`ChainRule::follows`]
    /// says of the eager form of the entry.
    fn follows(_entry: &E) -> bool {
        false
    }
}

/// What a view's scan of one chain is handed beside its rule ([`walk_chain`]):
/// the site its hook is told, and the inputs its first entry is read with.
pub struct ChainScan<const K: usize> {
    /// The chain each entry is shown to the hook as (docs/adr/0130).
    pub site: ChainSite,
    /// The inputs the first entry is read with: what the parent hands every
    /// entry, and 0 for each value the chain carries, which is 0 before its
    /// first entry.
    pub args: [u8; K],
}

/// Whether one more entry may be read under `left`, the entries the chain's
/// depth still allows (`None` where it has none), and that entry counted.
pub(crate) fn take_one(left: &mut Option<u32>) -> bool {
    match left {
        Some(0) => false,
        Some(n) => {
            *n -= 1;
            true
        }
        None => true,
    }
}

/// Judge how a chain ended, once its loop is left: the guards
/// `tlv_chain_guard` prints for every other backend.
///
/// A chain that ends with its frame owns the rest of it, so a byte left after
/// the loop stopped at the depth is an entry past it. A chain closed by an
/// entry flag is followed by other fields, so bytes left prove nothing; what
/// proves it unfinished is `more`, the last entry read declaring a successor
/// the loop did not take: with no byte left the frame ended inside the chain,
/// and with bytes left the chain ran past its depth. A chain with no depth has
/// no entry past it.
pub(crate) fn close_chain(
    reader: &mut SceReader<'_>,
    end: ChainEnd,
    depth: Option<u32>,
    more: bool,
) {
    match end {
        ChainEnd::Frame => {
            if depth.is_some() && reader.remaining() > 0 {
                reader.fail(CodecError::TooManyElements);
            }
        }
        ChainEnd::Flag => {
            if more && reader.remaining() == 0 {
                reader.fail(CodecError::NeedMoreBytes);
            }
            if more && depth.is_some() {
                reader.fail(CodecError::TooManyElements);
            }
        }
    }
}

/// Read the chain whose rule is `C` from `reader` into `list`, which the
/// list's `refill` hands over cleared, its first entry read with `args`: the
/// loop of every `<sce:tlv-chain>` of an eager decode, one function instantiated
/// at each chain's rule (docs/adr/0136).
///
/// Each entry is read until the chain ends, the loop being left where the
/// frame has failed, so a refusal makes no loop of reads that read nothing
/// (docs/adr/0128). An entry is held where the list has room and read on and
/// not held where it has none (docs/adr/0077 §7, [`push_keeping`]), and
/// `found[i]` is set where an entry read, held or not, is of the kind `i` of
/// the rule's [`ChainRule::Kinds`]: the kinds a gate after the chain reads.
/// Every refusal of the frame is recorded in `reader`; the answer is
/// `CapacityExceeded` where an entry was read and not held, which the list's
/// fill answers once the chain is read whole, and `Ok` otherwise.
#[inline(never)]
pub fn read_chain_into<'f, C, L, const K: usize>(
    reader: &mut SceReader<'f>,
    list: &mut L,
    mut args: [u8; K],
    found: &mut [bool],
) -> Result<(), CodecError>
where
    L: HeldList,
    C: ChainRule<'f, L::Entry, K>,
{
    let with_frame = C::END.with_frame();
    let mut left = C::DEPTH;
    let mut full = false;
    let mut more = false;
    while take_one(&mut left) {
        if with_frame && reader.remaining() == 0 {
            break;
        }
        if reader.failed() {
            break;
        }
        let entry = C::read(reader, &args);
        if !with_frame {
            more = C::follows(&entry);
        }
        C::carry(&entry, &mut args);
        let kinds = <C::Kinds as EntryKinds<L::Entry>>::COUNT;
        for (kind, seen) in found.iter_mut().enumerate().take(kinds) {
            *seen |= <C::Kinds as EntryKinds<L::Entry>>::is(&entry, kind);
        }
        push_keeping::<L, C::Kinds>(list, entry, &mut full);
        if !with_frame && !more {
            break;
        }
    }
    close_chain(reader, C::END, C::DEPTH, more);
    if full {
        Err(CodecError::CapacityExceeded)
    } else {
        Ok(())
    }
}

/// Scan the chain whose rule is `C` from `reader`, each entry read by the walk
/// `W` and shown to `hook`, and answer how many entries a walk of it yields:
/// the scan of every `<sce:tlv-chain>` of a view, one function instantiated at
/// each chain's walk and rule (docs/adr/0136).
///
/// Each entry is shown to `hook` right after it is read and before the next
/// is, as the bytes it occupies, and what the hook refuses is the frame's
/// refusal (docs/adr/0130); the entry's own body reads with the same hook. An
/// entry is counted only where its read was whole: one whose read recorded a
/// refusal is the zeros a total read returns, and the walk, which reads the
/// span again from its first entry, stops at it, so a count that held it would
/// be one the walk never reaches. The guards after the loop are
/// [`read_chain_into`]'s.
#[inline(never)]
pub fn walk_chain<'f, W, C, H, const K: usize>(
    reader: &mut SceReader<'f>,
    hook: &mut H,
    scan: ChainScan<K>,
) -> u32
where
    W: ListWalk<K>,
    C: ChainScanRule<W::Entry<'f>>,
    H: ChainHook,
{
    let ChainScan { site, mut args } = scan;
    let with_frame = C::END.with_frame();
    let mut left = C::DEPTH;
    let mut count: u32 = 0;
    let mut more = false;
    while take_one(&mut left) {
        if reader.failed() {
            break;
        }
        if with_frame && reader.remaining() == 0 {
            break;
        }
        let at = reader.mark();
        let entry = W::read_with(reader, &args, hook);
        show_entry(reader, hook, site, at);
        if !with_frame {
            more = C::follows(&entry);
        }
        W::advance(&entry, &mut args);
        count += u32::from(!reader.failed());
        if !with_frame && !more {
            break;
        }
    }
    close_chain(reader, C::END, C::DEPTH, more);
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::NoKinds;
    use crate::records::EntryRefused;

    /// An entry of one byte, and one more where its low bit is set; its high
    /// bit is the flag that says another entry follows.
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    struct Entry {
        head: u8,
        extra: Option<u8>,
        /// The value the chain carried into this entry.
        carried: u8,
    }

    fn read_entry(r: &mut SceReader<'_>, a: &[u8; 1]) -> Entry {
        let head = r.take_array::<1>()[0];
        let extra = if head & 1 == 1 {
            Some(r.take_array::<1>()[0])
        } else {
            None
        };
        Entry {
            head,
            extra,
            carried: a[0],
        }
    }

    fn follows(e: &Entry) -> bool {
        e.head & 0x80 != 0
    }

    /// The next entry is handed whether this one's second bit was set.
    fn carry_second_bit(e: &Entry, a: &mut [u8; 1]) {
        a[0] = u8::from(e.head & 0x02 != 0);
    }

    /// A rule of a chain of [`Entry`] that ends as `$end` at the depth
    /// `$depth`, its entries read by [`read_entry`] and its flag [`follows`],
    /// for the eager read and the scan alike, as a generated codec declares one
    /// for each chain. A chain that ends with its frame has the flag too, which
    /// the routines ask of no entry of it.
    macro_rules! rule {
        ($name:ident, $end:expr, $depth:expr) => {
            struct $name;

            impl ChainEnds for $name {
                const END: ChainEnd = $end;
                const DEPTH: Option<u32> = $depth;
            }

            impl<'f> ChainRule<'f, Entry, 1> for $name {
                type Kinds = NoKinds;

                fn read(r: &mut SceReader<'f>, a: &[u8; 1]) -> Entry {
                    read_entry(r, a)
                }

                fn follows(e: &Entry) -> bool {
                    follows(e)
                }
            }

            impl ChainScanRule<Entry> for $name {
                fn follows(e: &Entry) -> bool {
                    follows(e)
                }
            }
        };
    }

    rule!(FrameAt4, ChainEnd::Frame, Some(4));
    rule!(FrameAt2, ChainEnd::Frame, Some(2));
    rule!(FrameAny, ChainEnd::Frame, None);
    rule!(FlagAt4, ChainEnd::Flag, Some(4));
    rule!(FlagAt2, ChainEnd::Flag, Some(2));
    rule!(FlagAny, ChainEnd::Flag, None);

    type List = heapless::Vec<Entry, 4>;

    /// The list a chain was read into, the reader's verdict and what the
    /// routine answered.
    type Read = (List, Result<(), CodecError>, Result<(), CodecError>);

    fn read_all<C: ChainRule<'static, Entry, 1>>(bytes: &'static [u8]) -> Read {
        let mut r = SceReader::new(bytes);
        let mut list = List::new();
        let answer = read_chain_into::<C, _, 1>(&mut r, &mut list, [0], &mut []);
        (list, r.verdict(), answer)
    }

    fn heads(list: &List) -> heapless::Vec<u8, 4> {
        list.iter().map(|e| e.head).collect()
    }

    #[test]
    fn a_chain_to_the_end_of_its_frame_reads_every_entry() {
        let (list, verdict, answer) = read_all::<FrameAt4>(&[2, 3, 9, 4]);
        assert_eq!(heads(&list), [2, 3, 4]);
        assert_eq!(list[1].extra, Some(9));
        assert_eq!((verdict, answer), (Ok(()), Ok(())));
        let (list, verdict, _) = read_all::<FrameAt4>(&[]);
        assert!(list.is_empty());
        assert_eq!(verdict, Ok(()));
    }

    #[test]
    fn a_chain_that_ends_with_its_frame_reads_on_past_an_entry_whose_flag_is_clear() {
        // Every entry's flag is clear, and each is read all the same: the end
        // of the chain is its frame's, not an entry's.
        let (list, verdict, _) = read_all::<FrameAny>(&[0x02, 0x04, 0x06]);
        assert_eq!(heads(&list), [0x02, 0x04, 0x06]);
        assert_eq!(verdict, Ok(()));
        let mut r = SceReader::new(&[0x02, 0x04, 0x06]);
        assert_eq!(
            walk_chain::<Walk, FrameAny, _, 1>(&mut r, &mut (), scan()),
            3
        );
        assert_eq!(r.verdict(), Ok(()));
    }

    #[test]
    fn a_byte_past_the_depth_of_a_chain_that_owns_its_frame_is_too_many() {
        let (list, verdict, _) = read_all::<FrameAt2>(&[2, 4, 6]);
        assert_eq!(heads(&list), [2, 4]);
        assert_eq!(verdict, Err(CodecError::TooManyElements));
        // With no depth the chain reads to the end of its frame.
        let (list, verdict, _) = read_all::<FrameAny>(&[2, 4, 6]);
        assert_eq!(heads(&list), [2, 4, 6]);
        assert_eq!(verdict, Ok(()));
    }

    #[test]
    fn a_chain_closed_by_a_flag_ends_after_the_entry_whose_flag_is_clear() {
        let (list, verdict, _) = read_all::<FlagAt4>(&[0x82, 0x04, 0x06]);
        assert_eq!(heads(&list), [0x82, 0x04]);
        assert_eq!(verdict, Ok(()));
    }

    #[test]
    fn a_chain_closed_by_a_flag_is_short_or_too_long_where_its_last_flag_is_set() {
        // The frame ends after an entry that says another follows.
        let (_, verdict, _) = read_all::<FlagAt4>(&[0x82]);
        assert_eq!(verdict, Err(CodecError::NeedMoreBytes));
        let (_, verdict, _) = read_all::<FlagAny>(&[0x82]);
        assert_eq!(verdict, Err(CodecError::NeedMoreBytes));
        // The depth is reached with another entry announced and bytes left.
        let (list, verdict, _) = read_all::<FlagAt2>(&[0x82, 0x84, 0x06]);
        assert_eq!(heads(&list), [0x82, 0x84]);
        assert_eq!(verdict, Err(CodecError::TooManyElements));
        // A chain with no depth has no entry past it, and reads on to the flag.
        let (list, verdict, _) = read_all::<FlagAny>(&[0x82, 0x84, 0x06]);
        assert_eq!(heads(&list), [0x82, 0x84, 0x06]);
        assert_eq!(verdict, Ok(()));
        // An empty frame is short: the chain is one entry or more.
        let (_, verdict, _) = read_all::<FlagAt4>(&[]);
        assert_eq!(verdict, Err(CodecError::NeedMoreBytes));
    }

    /// How many entries [`Counted`] has read.
    static READS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

    /// [`FlagAny`], counting the entries it reads.
    struct Counted;

    impl ChainEnds for Counted {
        const END: ChainEnd = ChainEnd::Flag;
        const DEPTH: Option<u32> = None;
    }

    impl<'f> ChainRule<'f, Entry, 1> for Counted {
        type Kinds = NoKinds;

        fn read(r: &mut SceReader<'f>, a: &[u8; 1]) -> Entry {
            READS.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            read_entry(r, a)
        }

        fn follows(e: &Entry) -> bool {
            follows(e)
        }
    }

    #[test]
    fn an_entry_whose_read_is_refused_ends_the_loop() {
        // The entry announces another and asks for a byte the frame does not
        // have: its read is refused, and the loop reads nothing after it.
        READS.store(0, core::sync::atomic::Ordering::SeqCst);
        let (_, verdict, answer) = read_all::<Counted>(&[0x83]);
        assert_eq!((verdict, answer), (Err(CodecError::NeedMoreBytes), Ok(())));
        assert_eq!(READS.load(core::sync::atomic::Ordering::SeqCst), 1);
    }

    /// [`FrameAny`], carrying each entry's second bit to the next.
    struct Carried;

    impl ChainEnds for Carried {
        const END: ChainEnd = ChainEnd::Frame;
        const DEPTH: Option<u32> = None;
    }

    impl<'f> ChainRule<'f, Entry, 1> for Carried {
        type Kinds = NoKinds;

        fn read(r: &mut SceReader<'f>, a: &[u8; 1]) -> Entry {
            read_entry(r, a)
        }

        fn carry(e: &Entry, a: &mut [u8; 1]) {
            carry_second_bit(e, a);
        }
    }

    #[test]
    fn a_chain_carries_a_value_from_each_entry_to_the_next() {
        let (list, verdict, _) = read_all::<Carried>(&[2, 4, 6]);
        assert_eq!(verdict, Ok(()));
        let carried: heapless::Vec<u8, 4> = list.iter().map(|e| e.carried).collect();
        // The first entry is handed the start, 0; then each entry's second bit.
        assert_eq!(carried, [0, 1, 0]);
    }

    #[test]
    fn a_chain_starts_from_the_inputs_its_call_hands_it() {
        // What the parent hands every entry is the call's, not the rule's.
        let mut r = SceReader::new(&[2, 4]);
        let mut list = List::new();
        let answer = read_chain_into::<FrameAny, _, 1>(&mut r, &mut list, [7], &mut []);
        assert_eq!((r.verdict(), answer), (Ok(()), Ok(())));
        let carried: heapless::Vec<u8, 4> = list.iter().map(|e| e.carried).collect();
        assert_eq!(carried, [7, 7]);
    }

    #[test]
    fn a_chain_past_its_list_reads_on_and_answers_its_capacity_once_read_whole() {
        let mut r = SceReader::new(&[2, 4, 6]);
        let mut list: heapless::Vec<Entry, 1> = heapless::Vec::new();
        let answer = read_chain_into::<FrameAny, _, 1>(&mut r, &mut list, [0], &mut []);
        assert_eq!(answer, Err(CodecError::CapacityExceeded));
        // The frame is well formed and read to its end; the list holds what it can.
        assert_eq!(r.verdict(), Ok(()));
        assert_eq!(r.remaining(), 0);
        assert_eq!(list.len(), 1);
    }

    /// The kinds the gates on [`Kinded`] ask for: an entry whose head is 4, and
    /// one that says another follows.
    struct FourOrFollows;

    impl EntryKinds<Entry> for FourOrFollows {
        const COUNT: usize = 2;

        fn is(e: &Entry, kind: usize) -> bool {
            match kind {
                0 => e.head == 4,
                1 => follows(e),
                _ => false,
            }
        }
    }

    /// [`FrameAny`], its gates asking for [`FourOrFollows`].
    struct Kinded;

    impl ChainEnds for Kinded {
        const END: ChainEnd = ChainEnd::Frame;
        const DEPTH: Option<u32> = None;
    }

    impl<'f> ChainRule<'f, Entry, 1> for Kinded {
        type Kinds = FourOrFollows;

        fn read(r: &mut SceReader<'f>, a: &[u8; 1]) -> Entry {
            read_entry(r, a)
        }
    }

    #[test]
    fn the_kinds_of_every_entry_read_are_found_held_or_not() {
        let mut found = [false; 2];
        let mut r = SceReader::new(&[2, 4]);
        let mut list: heapless::Vec<Entry, 1> = heapless::Vec::new();
        let answer = read_chain_into::<Kinded, _, 1>(&mut r, &mut list, [0], &mut found);
        assert_eq!(answer, Err(CodecError::CapacityExceeded));
        // The second entry, the only one of the first kind, was read past the
        // list's capacity and noted all the same; no entry is of the second.
        assert_eq!(found, [true, false]);
        // A full list keeps an entry of a kind it held none of in place of one
        // of no kind the gates ask for (docs/adr/0065 §6 R6).
        assert_eq!(list[0].head, 4);
    }

    /// A walk of the same entries.
    struct Walk;

    impl ListWalk<1> for Walk {
        type Entry<'b> = Entry;

        fn read_with<'b, H: ChainHook>(r: &mut SceReader<'b>, a: &[u8; 1], _hook: &mut H) -> Entry {
            read_entry(r, a)
        }

        fn advance(e: &Entry, a: &mut [u8; 1]) {
            carry_second_bit(e, a);
        }
    }

    const SITE: ChainSite = ChainSite {
        codec: "chain",
        field: "entries",
    };

    fn scan() -> ChainScan<1> {
        ChainScan {
            site: SITE,
            args: [0],
        }
    }

    /// Admits an entry whose head is below `limit`, and keeps what it saw.
    struct Below {
        limit: u8,
        seen: heapless::Vec<u8, 4>,
    }

    impl ChainHook for Below {
        fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), EntryRefused> {
            assert_eq!(site, SITE);
            let head = entry.first().copied().unwrap_or(0);
            let _ = self.seen.push(head);
            if head < self.limit {
                Ok(())
            } else {
                Err(EntryRefused)
            }
        }
    }

    #[test]
    fn a_scan_counts_the_entries_a_walk_yields_and_judges_the_chain_as_the_read_does() {
        let mut r = SceReader::new(&[2, 3, 9, 4]);
        assert_eq!(
            walk_chain::<Walk, FrameAt4, _, 1>(&mut r, &mut (), scan()),
            3
        );
        assert_eq!(r.verdict(), Ok(()));
        let mut r = SceReader::new(&[2, 4, 6]);
        assert_eq!(
            walk_chain::<Walk, FrameAt2, _, 1>(&mut r, &mut (), scan()),
            2
        );
        assert_eq!(r.verdict(), Err(CodecError::TooManyElements));
        let mut r = SceReader::new(&[0x82, 0x04, 0x06]);
        assert_eq!(
            walk_chain::<Walk, FlagAt4, _, 1>(&mut r, &mut (), scan()),
            2
        );
        assert_eq!((r.verdict(), r.remaining()), (Ok(()), 1));
        let mut r = SceReader::new(&[0x82]);
        assert_eq!(
            walk_chain::<Walk, FlagAny, _, 1>(&mut r, &mut (), scan()),
            1
        );
        assert_eq!(r.verdict(), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn a_scan_counts_an_entry_only_where_its_read_was_whole() {
        // The second entry asks for a byte the frame does not have: it is read,
        // refused, and not counted, so the count is the entries a walk yields.
        let mut r = SceReader::new(&[2, 3]);
        assert_eq!(
            walk_chain::<Walk, FrameAny, _, 1>(&mut r, &mut (), scan()),
            1
        );
        assert_eq!(r.verdict(), Err(CodecError::NeedMoreBytes));
    }

    #[test]
    fn a_scan_shows_each_entry_to_its_hook_as_it_is_read() {
        let mut hook = Below {
            limit: 0xFF,
            seen: heapless::Vec::new(),
        };
        let mut r = SceReader::new(&[2, 3, 9, 4]);
        assert_eq!(
            walk_chain::<Walk, FrameAny, _, 1>(&mut r, &mut hook, scan()),
            3
        );
        assert_eq!(hook.seen, [2, 3, 4]);
        assert_eq!(r.verdict(), Ok(()));
    }

    #[test]
    fn what_the_hook_refuses_is_the_frames_refusal_and_ends_the_scan() {
        let mut hook = Below {
            limit: 3,
            seen: heapless::Vec::new(),
        };
        let mut r = SceReader::new(&[2, 3, 9, 4]);
        // The refused entry is not counted, and no entry after it is read.
        assert_eq!(
            walk_chain::<Walk, FrameAny, _, 1>(&mut r, &mut hook, scan()),
            1
        );
        assert_eq!(hook.seen, [2, 3]);
        assert_eq!(r.verdict(), Err(CodecError::ChainEntryRefused));
    }

    #[test]
    fn a_scan_reads_each_entry_with_what_the_walk_carried() {
        // The walk's `advance` carries each entry's second bit to the next; a
        // scan whose entries disagreed with a walk of them would count past
        // where the walk stops, so the two read with the same values.
        struct Strict;
        impl ListWalk<1> for Strict {
            type Entry<'b> = Entry;
            fn read_with<'b, H: ChainHook>(
                r: &mut SceReader<'b>,
                a: &[u8; 1],
                _hook: &mut H,
            ) -> Entry {
                let e = read_entry(r, a);
                // An entry handed 1 must have its second bit clear.
                if a[0] == 1 && e.head & 0x02 != 0 {
                    r.fail(CodecError::PresentIfMismatch);
                }
                e
            }
            fn advance(e: &Entry, a: &mut [u8; 1]) {
                carry_second_bit(e, a);
            }
        }
        let mut r = SceReader::new(&[2, 4, 2]);
        assert_eq!(
            walk_chain::<Strict, FrameAny, _, 1>(&mut r, &mut (), scan()),
            3
        );
        assert_eq!(r.verdict(), Ok(()));
        let mut r = SceReader::new(&[2, 2]);
        assert_eq!(
            walk_chain::<Strict, FrameAny, _, 1>(&mut r, &mut (), scan()),
            1
        );
        assert_eq!(r.verdict(), Err(CodecError::PresentIfMismatch));
    }
}
