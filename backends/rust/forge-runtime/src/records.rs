// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A sequence of frames read one record at a time, and the hook a chain's
//! entries are shown to as they are read (docs/adr/0130).
//!
//! [`Records`] reads the frames of one codec laid one after the other, through
//! one [`SceReader`]: the first refusal of any record is the sequence's verdict
//! (docs/adr/0128). A generated `{Codec}Records` peeks each record's tag and
//! hands the consumer a [`Record`], a borrow of the reader and nothing else; the
//! body is read only when the consumer asks for it ([`Record::read`]), so the
//! stack under the consumer's call holds the view of the one kind it reads. A
//! record handed out and not read is skipped by the next head, as the kind it
//! was handed out as and no other: the stack under the skip holds that kind's
//! view, never the view of every kind.
//!
//! [`ChainHook`] is shown each entry of a `<sce:tlv-chain>` right after it is
//! read, in the one read of the frame, and may refuse it ([`EntryRefused`]);
//! what it refuses is the frame's refusal, [`CodecError::ChainEntryRefused`].

use core::marker::PhantomData;

use crate::codec::CodecError;
use crate::reader::SceReader;

/// The chain an entry belongs to: the codec that declares it and the chain's
/// field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainSite {
    /// The codec's document name.
    pub codec: &'static str,
    /// The `<sce:tlv-chain>` field's id.
    pub field: &'static str,
}

/// A chain hook's refusal of an entry: the entry is well formed and the
/// consumer does not admit it. It is the one thing a hook can answer besides
/// admitting, so a hook cannot say a frame is short or malformed when the
/// frame is neither; why the hook refused is the hook's own to keep. The
/// frame's verdict says it as [`CodecError::ChainEntryRefused`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRefused;

impl From<EntryRefused> for CodecError {
    #[inline]
    fn from(_: EntryRefused) -> Self {
        CodecError::ChainEntryRefused
    }
}

/// What a generated read shows each chain entry to: `entry` is the bytes the
/// entry occupies in the frame, shown right after it is read and before the
/// next one is. An `Err` refuses the frame there, its verdict
/// [`CodecError::ChainEntryRefused`]; an entry admitted is kept or skipped as
/// the reading form keeps it.
///
/// `()` admits every entry, and a read with `()` is the read without a hook.
pub trait ChainHook {
    /// Admit or refuse one entry of the chain `site`.
    fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), EntryRefused>;
}

impl ChainHook for () {
    #[inline(always)]
    fn entry(&mut self, _site: ChainSite, _entry: &[u8]) -> Result<(), EntryRefused> {
        Ok(())
    }
}

/// Show `hook` the entry `reader` read since `at`, a [`SceReader::mark`] taken
/// before the entry, and record what it refuses. A frame that has failed shows
/// nothing: its entry is not one the frame holds.
#[inline(always)]
pub fn show_entry<H: ChainHook>(
    reader: &mut SceReader<'_>,
    hook: &mut H,
    site: ChainSite,
    at: usize,
) {
    if reader.failed() {
        return;
    }
    let entry = reader.since(at);
    if let Err(refused) = hook.entry(site, entry) {
        reader.fail(refused.into());
    }
}

/// How a record of one kind is read: the view of that kind, read from where the
/// record starts, its chains shown to `hook`.
pub trait RecordRead {
    /// The view a record of this kind is read as.
    type View<'f>;
    /// Read one record of this kind.
    fn read_with<'f, H: ChainHook>(reader: &mut SceReader<'f>, hook: &mut H) -> Self::View<'f>;
}

/// Read past one record of the kind `W`, as that kind's view, which is
/// dropped: the skip of a record handed out as `W` and not read. Out of line,
/// so the stack of the head that runs it holds no view.
#[cold]
#[inline(never)]
fn skip_record<W: RecordRead>(reader: &mut SceReader<'_>) {
    let _ = W::read_with(reader, &mut ());
}

/// How the head reads past a record handed out and not read: the skip of the
/// kind it was handed out as ([`skip_record`]).
type SkipRecord<'f> = fn(&mut SceReader<'f>);

/// The frames of one codec laid one after the other in `&'f [u8]`, read one
/// record at a time through one reader (docs/adr/0130).
#[derive(Debug, Clone)]
pub struct Records<'f> {
    reader: SceReader<'f>,
    whole: usize,
    /// The skip of the record handed out and not read, recorded when it was
    /// handed out; `None` when every record handed out was read.
    unread: Option<SkipRecord<'f>>,
}

impl<'f> Records<'f> {
    /// The records of `bytes`, none read.
    pub const fn new(bytes: &'f [u8]) -> Self {
        Self {
            reader: SceReader::new(bytes),
            whole: 0,
            unread: None,
        }
    }

    /// The bytes of the records read whole: where a refused record, or the
    /// first byte no record of the codec starts with, begins.
    pub const fn consumed(&self) -> usize {
        self.whole
    }

    /// The sequence's verdict: the first refusal of any record read, or `Ok`.
    pub fn verdict(&self) -> Result<(), CodecError> {
        self.reader.verdict()
    }

    /// The first byte of the next record, the record handed out before and not
    /// read skipped first, as the kind it was handed out as; `None` at the end
    /// of the bytes and once a record was refused, the skipped one among them.
    #[inline]
    pub fn head(&mut self) -> Option<u8> {
        if let Some(skip) = self.unread {
            skip(&mut self.reader);
            self.close();
        }
        if self.reader.failed() || self.reader.remaining() == 0 {
            return None;
        }
        self.reader.rest().first().copied()
    }

    /// Refuse the sequence at the head with `refused`, its verdict: a record
    /// whose tag names an arm this build leaves out. The bytes read whole stay
    /// where they were, and the next head is `None`.
    pub fn fail(&mut self, refused: CodecError) {
        self.reader.fail(refused);
    }

    /// Hand out the record at the head, of the kind `W` its tag says. Until it
    /// is read, the next head skips it as a `W`.
    #[inline]
    pub fn record<W: RecordRead>(&mut self) -> Record<'_, 'f, W> {
        self.unread = Some(skip_record::<W>);
        Record {
            records: self,
            kind: PhantomData,
        }
    }

    /// The record handed out has been read: the bytes read whole move past it
    /// where it was not refused.
    fn close(&mut self) {
        self.unread = None;
        if !self.reader.failed() {
            self.whole = self.reader.position();
        }
    }
}

/// One record of a [`Records`], not yet read: a borrow of the reader and
/// nothing else, so handing it out costs one pointer whatever its kind.
pub struct Record<'r, 'f, W> {
    records: &'r mut Records<'f>,
    kind: PhantomData<fn() -> W>,
}

impl<'f, W: RecordRead> Record<'_, 'f, W> {
    /// The record's first byte, the one its kind was told by.
    pub fn header(&self) -> u8 {
        self.records.reader.rest().first().copied().unwrap_or(0)
    }

    /// Read the record's view, admitting every chain entry.
    #[inline]
    pub fn read(self) -> W::View<'f> {
        self.read_with(&mut ())
    }

    /// Read the record's view, each chain entry shown to `hook`.
    #[inline]
    pub fn read_with<H: ChainHook>(self, hook: &mut H) -> W::View<'f> {
        let view = W::read_with(&mut self.records.reader, hook);
        self.records.close();
        view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record of a tag byte and a count of bytes after it.
    struct Counted;

    impl RecordRead for Counted {
        type View<'f> = &'f [u8];

        fn read_with<'f, H: ChainHook>(r: &mut SceReader<'f>, hook: &mut H) -> &'f [u8] {
            let at = r.mark();
            let head = r.take_array::<2>();
            let body = r.take_slice(usize::from(head[1]));
            show_entry(
                r,
                hook,
                ChainSite {
                    codec: "counted",
                    field: "body",
                },
                at,
            );
            body
        }
    }

    /// A record of a tag byte and one byte after it, whatever that byte says.
    struct Pair;

    impl RecordRead for Pair {
        type View<'f> = &'f [u8; 2];

        fn read_with<'f, H: ChainHook>(r: &mut SceReader<'f>, _hook: &mut H) -> &'f [u8; 2] {
            r.take_array::<2>()
        }
    }

    /// The records of `bytes` whose tag is below 0x10, each read.
    fn read_all(bytes: &[u8]) -> (usize, Result<(), CodecError>, usize) {
        let mut records = Records::new(bytes);
        let mut read = 0;
        while let Some(tag) = records.head() {
            if tag >= 0x10 {
                break;
            }
            records.record::<Counted>().read();
            read += 1;
        }
        (read, records.verdict(), records.consumed())
    }

    #[test]
    fn records_are_read_one_after_the_other_to_the_end() {
        assert_eq!(read_all(&[1, 1, 9, 2, 0]), (2, Ok(()), 5));
        assert_eq!(read_all(&[]), (0, Ok(()), 0));
    }

    #[test]
    fn a_tag_no_record_starts_with_ends_the_sequence_before_it() {
        assert_eq!(read_all(&[1, 0, 0x20, 7]), (1, Ok(()), 2));
    }

    #[test]
    fn the_first_refusal_ends_the_sequence_where_the_last_whole_record_ended() {
        let (read, verdict, consumed) = read_all(&[1, 0, 2, 5, 1]);
        assert_eq!(
            (read, verdict, consumed),
            (2, Err(CodecError::NeedMoreBytes), 2)
        );
    }

    #[test]
    fn a_record_handed_out_and_not_read_is_skipped() {
        let bytes = [1u8, 1, 9, 2, 1, 8];
        let mut records = Records::new(&bytes);
        assert_eq!(records.head(), Some(1));
        let first = records.record::<Counted>();
        assert_eq!(first.header(), 1);
        assert_eq!(records.head(), Some(2));
        assert_eq!(records.consumed(), 3);
        assert_eq!(records.record::<Counted>().read(), &[8]);
        assert_eq!(records.head(), None);
        assert_eq!(records.consumed(), 6);
    }

    #[test]
    fn a_record_not_read_is_skipped_as_the_kind_it_was_handed_out_as() {
        // As a `Pair` the first record is two bytes; as a `Counted` it would
        // be five, and the head after it would be the end of the bytes.
        let bytes = [1u8, 3, 7, 0, 0];
        let mut records = Records::new(&bytes);
        assert_eq!(records.head(), Some(1));
        let _unread = records.record::<Pair>();
        assert_eq!(records.head(), Some(7));
        assert_eq!(records.consumed(), 2);
        // The same bytes handed out as a `Counted` are skipped as one.
        let mut records = Records::new(&bytes);
        assert_eq!(records.head(), Some(1));
        let _unread = records.record::<Counted>();
        assert_eq!(records.head(), None);
        assert_eq!((records.verdict(), records.consumed()), (Ok(()), 5));
    }

    #[test]
    fn a_malformed_record_skipped_unread_refuses_the_sequence() {
        // The second record says five bytes follow and two do.
        let bytes = [1u8, 0, 2, 5, 9, 9];
        let mut records = Records::new(&bytes);
        assert_eq!(records.head(), Some(1));
        records.record::<Counted>().read();
        assert_eq!(records.head(), Some(2));
        let _unread = records.record::<Counted>();
        assert_eq!(records.head(), None);
        assert_eq!(records.verdict(), Err(CodecError::NeedMoreBytes));
        assert_eq!(records.consumed(), 2);
        assert_eq!(records.head(), None);
    }

    #[test]
    fn a_hooks_refusal_is_its_own_kind_of_verdict() {
        assert_eq!(
            CodecError::from(EntryRefused),
            CodecError::ChainEntryRefused
        );
        let mut r = SceReader::new(&[1u8, 0]);
        let at = r.mark();
        r.take_array::<2>();
        let mut hook = Below { limit: 0, seen: 0 };
        show_entry(
            &mut r,
            &mut hook,
            ChainSite {
                codec: "counted",
                field: "body",
            },
            at,
        );
        assert_eq!(hook.seen, 1);
        assert_eq!(r.verdict(), Err(CodecError::ChainEntryRefused));
    }

    #[test]
    fn a_record_is_one_pointer() {
        assert_eq!(
            core::mem::size_of::<Record<'_, '_, Counted>>(),
            core::mem::size_of::<usize>()
        );
    }

    #[test]
    fn a_sequence_refused_at_its_head_keeps_what_it_read_whole() {
        let bytes = [1u8, 0, 2, 0];
        let mut records = Records::new(&bytes);
        assert_eq!(records.head(), Some(1));
        records.record::<Counted>().read();
        assert_eq!(records.head(), Some(2));
        records.fail(CodecError::VariantArmNotBuilt);
        assert_eq!(records.head(), None);
        assert_eq!(records.verdict(), Err(CodecError::VariantArmNotBuilt));
        assert_eq!(records.consumed(), 2);
    }

    /// Admits an entry whose first byte is below `limit`, and counts what it saw.
    struct Below {
        limit: u8,
        seen: usize,
    }

    impl ChainHook for Below {
        fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), EntryRefused> {
            assert_eq!(site.codec, "counted");
            self.seen += 1;
            match entry.first() {
                Some(&b) if b < self.limit => Ok(()),
                _ => Err(EntryRefused),
            }
        }
    }

    #[test]
    fn a_hook_sees_each_entry_whole_and_its_refusal_is_the_verdict() {
        let bytes = [1u8, 1, 9, 3, 0];
        let mut records = Records::new(&bytes);
        let mut hook = Below { limit: 2, seen: 0 };
        while records.head().is_some() {
            records.record::<Counted>().read_with(&mut hook);
        }
        assert_eq!(hook.seen, 2);
        assert_eq!(records.verdict(), Err(CodecError::ChainEntryRefused));
        assert_eq!(records.consumed(), 3);
    }

    #[test]
    fn a_failed_frame_shows_the_hook_nothing() {
        let mut r = SceReader::new(&[1u8]);
        let at = r.mark();
        r.take_array::<2>();
        let mut hook = Below { limit: 0, seen: 0 };
        show_entry(
            &mut r,
            &mut hook,
            ChainSite {
                codec: "counted",
                field: "body",
            },
            at,
        );
        assert_eq!(hook.seen, 0);
        assert_eq!(r.verdict(), Err(CodecError::NeedMoreBytes));
    }
}
