//! Binary wire codec for protocol messages, and the optional serde and JSON bridges.
//!
//! Spec §11 (opaque client payloads), §13.1 (bounded view-change suffix) and decisions
//! W1, W3, W5, W4.
//!
//! The normative encoding is binary big-endian with a 20-byte header
//! `(tag: u32, era: u32, view: u32, slot: u64)` and no bit packing. Binary is normative
//! rather than merely available because the §13.1 suffix budget must be computable
//! exactly: a primary deciding how much history fits in a report needs the encoded size
//! of a candidate suffix to be a function of the suffix, not of the serializer's
//! whitespace or number formatting.
//!
//! There is no `MAX_DATAGRAM` here and there never will be (decision W5). Encoders
//! report the size they require and decoders report incomplete input; the host owns
//! every packetization decision, because only the host knows its MTU, its transport,
//! and whether it is willing to fragment.
//!
//! `serde` derives sit behind the `serde` feature so a host may impose its own
//! encoding, and the `maelstrom` feature adds a binary-to-JSON debug bridge. The bridge
//! exists so that the binary path is the one under test while the failure output is
//! readable; a JSON-native path would test the wrong codec.
//!
//! # What this module is, and what it is deliberately not
//!
//! This is the codec *substrate*: byte-level primitives, the [`Pack`] and [`Unpack`]
//! traits, the error taxonomy, the [`Tag`] discriminant table, and [`Header`]. It
//! defines no message body. A body defined here would have to be redefined once
//! `Prepare` has a log entry to carry and `StartView` has configuration evidence to
//! carry; each later message body owns its own encoding and adds its own round trip through the
//! traits below.
//!
//! # Zero allocation, both directions
//!
//! The primary API packs into a host-owned `&mut [u8]` and reads from a `&[u8]`. There
//! is no `Vec` in any signature and no allocation on either path. That is what lets one
//! implementation serve a C ABI caller with a stack buffer, a LuaJIT FFI buffer and a
//! WASM linear-memory slice, without a copy at the boundary or a second encoder to keep
//! in step.
//!
//! # Totality
//!
//! [`UnpackCursor`] returns `Ok` or `Err` on every input, adversarial included, in
//! release builds as well as debug. No `debug_assert` stands in for a check, there is no
//! `as` cast between integer widths, no `wrapping_*`, and no indexing that can panic:
//! every read goes through a bounds-checked accessor and a failed read becomes
//! [`UnpackError::Incomplete`]. A decoder that aborts the host process on a hostile
//! datagram is a denial-of-service vector, not a safety mechanism.

use core::mem::size_of;

use crate::ids::{Era, NodeId, OperationId, Slot, Tick, View, ViewId};

/// Byte width of the `u32` length prefix in front of an opaque payload.
const LENGTH_PREFIX_LEN: usize = 4;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a decode could not complete.
///
/// The distinction is load-bearing (W5): `Incomplete` means "give me more bytes and ask
/// again", `Malformed` means "these bytes are not a message and no amount of additional
/// input will make them one". A host reassembling a fragmented datagram must be able to
/// tell those apart without guessing, and a peer that sends `Malformed` input must be
/// droppable without faulting the node.
///
/// Collapsing the two forces one of two bad choices: treat every short read as garbage,
/// which makes fragmentation impossible, or treat every garbage read as short, which
/// lets a hostile peer hold a reassembly buffer open indefinitely.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnpackError {
    /// Ran out of input.
    ///
    /// `needed` is the **total** byte count required to make progress past the point
    /// that failed, measured from the start of the buffer handed to the cursor — not the
    /// remaining shortfall. A host can therefore use it directly as a reservation size
    /// without having to remember how much it already had.
    Incomplete {
        /// Total bytes required, measured from the start of the buffer.
        needed: usize,
    },
    /// The bytes are not a message. Additional input cannot help.
    Malformed(Malformed),
}

/// The ways a byte string fails to be a message.
///
/// Exhaustive and `Copy`, so a host may log or count occurrences without allocating and
/// without a catch-all arm that would silently absorb a new failure mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// The tag field held a value outside the [`Tag`] table. The offending value is
    /// carried so a host can log what it dropped; `0` is reserved and lands here.
    UnknownTag(u32),
    /// A length prefix declared a total size not representable on this target, so no
    /// input could ever satisfy it. Merely exceeding the bytes currently in hand is
    /// [`UnpackError::Incomplete`], not this — see [`UnpackCursor::opaque`].
    LengthPrefixOverflow,
    /// A complete value was decoded and input remained. A message with a suffix is not a
    /// message: ignoring the surplus would hide a framing bug in the host's reassembly,
    /// which is precisely the bug hardest to find from the far end.
    TrailingBytes {
        /// Bytes left unread after a complete decode.
        unread: usize,
    },
    /// A field required to be zero was not. Reserved space that a decoder tolerates is
    /// reserved space a later revision cannot use.
    ReservedFieldNonZero,
    /// A newtype's checked domain rejected the decoded value — a `bool` byte other than
    /// `0x00` or `0x01`, for instance. Accepting a non-canonical encoding gives one
    /// message two byte strings and forfeits any later argument that a message has a
    /// canonical form.
    OutOfDomain,
}

/// Why an encode could not complete.
///
/// One variant, because a fixed-width big-endian encoding (W4) has exactly one way to
/// fail: the destination is too small. There is no allocation to fail and no in-range
/// value that cannot be expressed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackError {
    /// The caller's buffer is smaller than [`Pack::packed_len`].
    ///
    /// **Nothing was written.** The bounds check happens in [`PackWriter::new`], before
    /// the buffer is touched, so a host may retry into a larger buffer or reuse the same
    /// scratch buffer without clearing it. Losing that guarantee would push a clearing
    /// cost into every host that budgets optimistically, which under §13.1 is the normal
    /// case rather than the exception.
    BufferTooSmall {
        /// Bytes [`Pack::packed_len`] reported.
        needed: usize,
        /// Bytes the caller supplied.
        provided: usize,
    },
}

// ---------------------------------------------------------------------------
// PackWriter
// ---------------------------------------------------------------------------

/// A cursor writing big-endian fields into a host-owned buffer.
///
/// Every method is infallible, because [`PackWriter::new`] has already established that
/// the buffer holds the declared requirement. That is deliberate: it moves the single
/// bounds check to one place, removes a `?` from every field write, and makes the
/// "nothing was written on failure" guarantee of [`PackError::BufferTooSmall`] a
/// property of the type rather than a discipline each [`Pack`] impl must remember. The
/// cost is that an impl whose `packed_len` under-reports panics instead of returning an
/// error — which is the right trade, because such an impl violates the normative W3
/// contract and the defect is in this crate rather than in the host's input.
pub struct PackWriter<'a> {
    buf: &'a mut [u8],
    at: usize,
}

impl<'a> PackWriter<'a> {
    /// Claims `needed` bytes of `buf`, or refuses without writing anything.
    ///
    /// `needed` is normally [`Pack::packed_len`] of the value about to be written.
    ///
    /// # Errors
    ///
    /// [`PackError::BufferTooSmall`] when `buf` is shorter than `needed`.
    pub fn new(buf: &'a mut [u8], needed: usize) -> Result<Self, PackError> {
        let provided = buf.len();
        if provided < needed {
            return Err(PackError::BufferTooSmall { needed, provided });
        }
        Ok(Self { buf, at: 0 })
    }

    /// Bytes written so far. The value a `pack_into` returns.
    #[must_use]
    pub fn written(&self) -> usize {
        self.at
    }

    /// Copies `src` at the cursor and advances.
    ///
    /// # Panics
    ///
    /// If the requirement declared to [`PackWriter::new`] was smaller than the bytes
    /// actually written. That is a broken [`Pack::packed_len`] in this crate, not a host
    /// error, and it is a defect worth surfacing loudly rather than encoding into a
    /// `Result` that every call site would have to thread.
    fn put(&mut self, src: &[u8]) {
        let end = self
            .at
            .checked_add(src.len())
            .expect("PackWriter offset overflow: packed_len() under-reported");
        let dst = self
            .buf
            .get_mut(self.at..end)
            .expect("PackWriter overrun: packed_len() under-reported");
        dst.copy_from_slice(src);
        self.at = end;
    }

    /// Writes one byte.
    pub fn u8(&mut self, value: u8) {
        self.put(&[value]);
    }

    /// Writes a big-endian `u16`.
    pub fn u16(&mut self, value: u16) {
        self.put(&value.to_be_bytes());
    }

    /// Writes a big-endian `u32`.
    pub fn u32(&mut self, value: u32) {
        self.put(&value.to_be_bytes());
    }

    /// Writes a big-endian `u64`.
    pub fn u64(&mut self, value: u64) {
        self.put(&value.to_be_bytes());
    }

    /// Writes a big-endian `u128`.
    pub fn u128(&mut self, value: u128) {
        self.put(&value.to_be_bytes());
    }

    /// Writes `0x00` or `0x01`.
    ///
    /// Canonical by construction: the decoder rejects every other byte, so the pair
    /// `(pack, unpack)` is a bijection on one byte rather than a surjection onto it.
    pub fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    /// Writes 16 opaque bytes verbatim.
    pub fn bytes16(&mut self, value: &[u8; 16]) {
        self.put(value);
    }

    /// Writes a `u32` big-endian length prefix and then `value` verbatim.
    ///
    /// This is how host client payloads travel (§11): the core never inspects them, so
    /// they get a length and no interpretation.
    ///
    /// # Panics
    ///
    /// If `value.len()` exceeds `u32::MAX`. A caller sized its buffer with
    /// [`opaque_packed_len`], which refuses such a payload first, so reaching this means
    /// the caller budgeted with something else.
    pub fn opaque(&mut self, value: &[u8]) {
        let len = u32::try_from(value.len())
            .expect("opaque payload exceeds u32::MAX; opaque_packed_len() rejects it first");
        self.u32(len);
        self.put(value);
    }
}

/// Exact encoded length of an opaque payload, or `None` if it is not encodable.
///
/// `None` when the payload exceeds what the `u32` prefix can express. Reported rather
/// than truncated or saturated: a silently shortened payload is a corrupted client
/// operation, and the core has no authority to shorten one.
#[must_use]
pub fn opaque_packed_len_checked(payload: &[u8]) -> Option<usize> {
    if u32::try_from(payload.len()).is_err() {
        return None;
    }
    payload.len().checked_add(LENGTH_PREFIX_LEN)
}

/// Exact encoded length of an opaque payload.
///
/// # Panics
///
/// If the payload cannot be framed in a `u32`-prefixed field. Use
/// [`opaque_packed_len_checked`] where a payload of untrusted size is possible; this
/// convenience exists because a host that has already framed a datagram knows its
/// payload is orders of magnitude below the limit.
#[must_use]
pub fn opaque_packed_len(payload: &[u8]) -> usize {
    opaque_packed_len_checked(payload).expect("opaque payload length must fit a u32 prefix")
}

/// Packs an opaque payload into `buf`, returning the bytes written.
///
/// A free function rather than a [`Pack`] impl on `&[u8]`: `packed_len` on a bare slice
/// would invite a caller to treat the slice as a message. It is a field, and only a body
/// knows where a field belongs.
///
/// # Errors
///
/// [`PackError::BufferTooSmall`] when `buf` is shorter than [`opaque_packed_len`]; `buf`
/// is untouched in that case.
pub fn pack_opaque_into(payload: &[u8], buf: &mut [u8]) -> Result<usize, PackError> {
    // An unencodable payload reports `usize::MAX` rather than gaining an error variant
    // reachable only with a payload larger than the prefix can name. The caller sees a
    // requirement it cannot satisfy, which is the truth.
    let needed = opaque_packed_len_checked(payload).unwrap_or(usize::MAX);
    let mut writer = PackWriter::new(buf, needed)?;
    writer.opaque(payload);
    Ok(writer.written())
}

// ---------------------------------------------------------------------------
// UnpackCursor
// ---------------------------------------------------------------------------

/// A cursor reading big-endian fields out of a borrowed buffer.
///
/// Total on every input. No method panics, in release or in debug, for any byte string
/// including one chosen by an adversary. Reads that run off the end return
/// [`UnpackError::Incomplete`] with the total requirement; reads that succeed but yield a
/// value outside a field's domain return [`UnpackError::Malformed`].
///
/// The cursor does not roll back on error, and deliberately offers no way to. A caller
/// that wants to retry with more bytes constructs a fresh cursor over the longer buffer,
/// which is the only correct thing to do anyway: a longer buffer may reinterpret a
/// length prefix that the prefix-length read got from truncated input.
pub struct UnpackCursor<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> UnpackCursor<'a> {
    /// Wraps a buffer. Reading starts at offset zero.
    #[must_use]
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, at: 0 }
    }

    /// Bytes consumed so far.
    #[must_use]
    pub fn read(&self) -> usize {
        self.at
    }

    /// Bytes not yet consumed.
    #[must_use]
    pub fn remaining(&self) -> usize {
        // `at` never exceeds `buf.len()`, because every advance goes through `take`,
        // which advances only after a successful bounds-checked slice. `saturating_sub`
        // rather than `-` so that the totality claim holds by construction and not by
        // that argument.
        self.buf.len().saturating_sub(self.at)
    }

    /// Asserts the whole buffer was consumed.
    ///
    /// Every top-level decode calls this, which is why [`Unpack::unpack_from`] provides
    /// it rather than leaving it to each implementation. A message with trailing garbage
    /// is not a message.
    ///
    /// # Errors
    ///
    /// [`Malformed::TrailingBytes`] when bytes remain.
    pub fn finish(&self) -> Result<(), UnpackError> {
        let unread = self.remaining();
        if unread == 0 {
            Ok(())
        } else {
            Err(UnpackError::Malformed(Malformed::TrailingBytes { unread }))
        }
    }

    /// Borrows the next `len` bytes and advances, or reports the total requirement.
    ///
    /// The one place in this module that computes an offset, therefore the one place a
    /// panic could be introduced. `checked_add` and `get` between them make that
    /// unreachable without relying on a `debug_assert`.
    fn take(&mut self, len: usize) -> Result<&'a [u8], UnpackError> {
        let end = match self.at.checked_add(len) {
            Some(end) => end,
            // The requirement is not expressible as an offset on this target, so no
            // input can satisfy it. Only reachable from a length prefix: the fixed-width
            // primitives ask for at most 16 bytes.
            None => return Err(UnpackError::Malformed(Malformed::LengthPrefixOverflow)),
        };
        match self.buf.get(self.at..end) {
            Some(slice) => {
                self.at = end;
                Ok(slice)
            }
            None => Err(UnpackError::Incomplete { needed: end }),
        }
    }

    /// Reads a fixed-width array.
    ///
    /// Generic over the width so each primitive below is a one-line conversion carrying
    /// no slice-length assumption of its own, and so a width appears exactly once per
    /// primitive.
    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], UnpackError> {
        let before = self.at;
        let slice = self.take(N)?;
        // `take` returned exactly `N` bytes, so the conversion cannot fail. It is
        // written as a fallible conversion rather than an `unwrap` so that no path here
        // depends on an invariant the reader has to verify by hand; the error arm is the
        // same requirement `take` would have reported.
        <[u8; N]>::try_from(slice).map_err(|_| UnpackError::Incomplete {
            needed: before.saturating_add(N),
        })
    }

    /// Reads one byte.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if the buffer is exhausted.
    pub fn u8(&mut self) -> Result<u8, UnpackError> {
        self.take_array::<1>().map(u8::from_be_bytes)
    }

    /// Reads a big-endian `u16`.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than 2 bytes remain.
    pub fn u16(&mut self) -> Result<u16, UnpackError> {
        self.take_array::<2>().map(u16::from_be_bytes)
    }

    /// Reads a big-endian `u32`.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than 4 bytes remain.
    pub fn u32(&mut self) -> Result<u32, UnpackError> {
        self.take_array::<4>().map(u32::from_be_bytes)
    }

    /// Reads a big-endian `u64`.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than 8 bytes remain.
    pub fn u64(&mut self) -> Result<u64, UnpackError> {
        self.take_array::<8>().map(u64::from_be_bytes)
    }

    /// Reads a big-endian `u128`.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than 16 bytes remain.
    pub fn u128(&mut self) -> Result<u128, UnpackError> {
        self.take_array::<16>().map(u128::from_be_bytes)
    }

    /// Reads a canonical `bool`.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if the buffer is exhausted;
    /// [`Malformed::OutOfDomain`] for any byte other than `0x00` or `0x01`.
    pub fn bool(&mut self) -> Result<bool, UnpackError> {
        match self.u8()? {
            0x00 => Ok(false),
            0x01 => Ok(true),
            _ => Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        }
    }

    /// Reads 16 opaque bytes.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than 16 bytes remain.
    pub fn bytes16(&mut self) -> Result<[u8; 16], UnpackError> {
        self.take_array::<16>()
    }

    /// Reads a `u32`-prefixed opaque payload, borrowed from the input.
    ///
    /// Borrowed rather than copied, so a client payload crosses the codec without an
    /// allocation. That is what makes the decode path usable from a WASM linear-memory
    /// slice or a LuaJIT FFI buffer with no bounce buffer, and the core never inspects
    /// the bytes in any case (§11).
    ///
    /// # The `Incomplete` / `LengthPrefixOverflow` rule
    ///
    /// A declared length that merely exceeds the bytes currently in hand is
    /// [`UnpackError::Incomplete`], because the host may still be reassembling and the
    /// core was never told the datagram size (W5); reporting `Malformed` there would make
    /// a decoder drop a peer over a fragment boundary.
    /// [`Malformed::LengthPrefixOverflow`] is reserved for a declared length whose
    /// implied *total* offset is not representable as a `usize`, which no input on this
    /// target can ever satisfy. On a 64-bit target a `u32` prefix cannot reach that
    /// condition, so the arm is a correctness guard for narrower targets rather than a
    /// case a 64-bit host will observe; the point is that the arithmetic is checked and
    /// not an unchecked add.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if the prefix or the payload is short;
    /// [`Malformed::LengthPrefixOverflow`] if the declared total is unrepresentable.
    pub fn opaque(&mut self) -> Result<&'a [u8], UnpackError> {
        let declared = self.u32()?;
        // `u32 -> usize` widens on every target this crate supports, and `try_from`
        // states that rather than assuming it. A 16-bit target legitimately fails here,
        // and the failure is the same "no input can satisfy this" condition.
        let len = usize::try_from(declared)
            .map_err(|_| UnpackError::Malformed(Malformed::LengthPrefixOverflow))?;
        self.take(len)
    }

    /// Reads a fixed-width field required to be zero.
    ///
    /// Provided so that reserved space stays reserved: a decoder that tolerates a
    /// non-zero reserved field has already spent the space, because a later revision
    /// cannot then distinguish an old sender's garbage from a new sender's meaning.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] if fewer than `len` bytes remain;
    /// [`Malformed::ReservedFieldNonZero`] if any byte is non-zero.
    pub fn reserved(&mut self, len: usize) -> Result<(), UnpackError> {
        let bytes = self.take(len)?;
        if bytes.iter().all(|&byte| byte == 0) {
            Ok(())
        } else {
            Err(UnpackError::Malformed(Malformed::ReservedFieldNonZero))
        }
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Encodes a value into the normative binary form.
pub trait Pack {
    /// Exact encoded length in bytes.
    ///
    /// **Normative (W3).** Callers budget with this, and it must equal the number of
    /// bytes [`Pack::pack`] writes, for every value, always. An upper bound is not
    /// acceptable: §13.1 constructs a suffix by adding entries until a budget would be
    /// exceeded, and with an inexact length that loop becomes a search with a re-encode
    /// per candidate. It is also why W4 forbids varints — a value-dependent width makes
    /// the budget a function of the data rather than of the shape.
    fn packed_len(&self) -> usize;

    /// Writes the value at the writer's cursor.
    ///
    /// Infallible: the writer's construction already established the space. An
    /// implementation that writes a number of bytes other than [`Pack::packed_len`]
    /// violates the W3 contract, and the writer panics rather than truncating.
    fn pack(&self, w: &mut PackWriter<'_>);

    /// Bounds-checks once, then packs. Returns the bytes written.
    ///
    /// # Errors
    ///
    /// [`PackError::BufferTooSmall`] when `buf` is shorter than [`Pack::packed_len`]. In
    /// that case `buf` is untouched, so a host may reuse a scratch buffer across a
    /// budget-exceeded retry without clearing it.
    fn pack_into(&self, buf: &mut [u8]) -> Result<usize, PackError> {
        let needed = self.packed_len();
        let mut writer = PackWriter::new(buf, needed)?;
        self.pack(&mut writer);
        Ok(writer.written())
    }
}

/// Decodes a value from the normative binary form.
pub trait Unpack: Sized {
    /// Reads one value at the cursor, leaving the cursor positioned after it.
    ///
    /// # Errors
    ///
    /// [`UnpackError::Incomplete`] when the input is short, [`UnpackError::Malformed`]
    /// when it is not a value of this type. Never panics, for any input.
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError>;

    /// Decodes from a complete buffer and rejects trailing bytes.
    ///
    /// The trailing-byte rejection is here rather than inside each [`Unpack::unpack`], so
    /// that a nested field never demands to be the last thing in the buffer while a
    /// top-level message always does.
    ///
    /// # Errors
    ///
    /// As [`Unpack::unpack`], plus [`Malformed::TrailingBytes`] when the buffer holds
    /// more than one value.
    fn unpack_from(buf: &[u8]) -> Result<Self, UnpackError> {
        let mut cursor = UnpackCursor::new(buf);
        let value = Self::unpack(&mut cursor)?;
        cursor.finish()?;
        Ok(value)
    }
}

/// Generates [`Pack`]/[`Unpack`] for a fixed-width scalar, so the width, the endianness
/// and the length claim are each stated once. Three copies of `4` in a hand-written impl
/// are three chances for `packed_len` to disagree with `pack`.
macro_rules! impl_scalar {
    ($ty:ty, $len:expr, $put:ident, $get:ident) => {
        impl Pack for $ty {
            fn packed_len(&self) -> usize {
                $len
            }

            fn pack(&self, w: &mut PackWriter<'_>) {
                w.$put(*self);
            }
        }

        impl Unpack for $ty {
            fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
                c.$get()
            }
        }
    };
}

impl_scalar!(u8, 1, u8, u8);
impl_scalar!(u16, 2, u16, u16);
impl_scalar!(u32, 4, u32, u32);
impl_scalar!(u64, 8, u64, u64);
impl_scalar!(u128, 16, u128, u128);
impl_scalar!(bool, 1, bool, bool);

/// Generates [`Pack`]/[`Unpack`] for a `#[repr(transparent)]` newtype over a scalar the
/// macro above already covers, so no identifier can acquire an endianness of its own.
macro_rules! impl_newtype {
    ($ty:ty, $inner:ty) => {
        impl Pack for $ty {
            fn packed_len(&self) -> usize {
                self.0.packed_len()
            }

            fn pack(&self, w: &mut PackWriter<'_>) {
                self.0.pack(w);
            }
        }

        impl Unpack for $ty {
            fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
                <$inner as Unpack>::unpack(c).map(Self)
            }
        }
    };
}

impl_newtype!(NodeId, u32);
impl_newtype!(Era, u32);
impl_newtype!(View, u32);
impl_newtype!(Slot, u64);
impl_newtype!(Tick, u64);

// `OperationId` (§11.1) is two `u64` words, most significant first, each big-endian —
// W4, like every other integer on this wire. The byte order is a decision this module
// owns (a host reading a hex dump sees the identity in the order it wrote it), and it
// is pinned by a golden vector in `tests/wire_contract.rs`, because a round trip cannot
// detect a symmetric endianness mistake.
impl Pack for OperationId {
    fn packed_len(&self) -> usize {
        self.msb.packed_len() + self.lsb.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.msb.pack(w);
        self.lsb.pack(w);
    }
}

impl Unpack for OperationId {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let msb = u64::unpack(c)?;
        let lsb = u64::unpack(c)?;
        Ok(OperationId { msb, lsb })
    }
}

impl Pack for ViewId {
    fn packed_len(&self) -> usize {
        self.era.packed_len() + self.view.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        // Era before view, matching the W1 header layout, so a `ViewId` nested in a body
        // is byte-identical to the pair in the header and a host has one field order to
        // learn rather than two.
        self.era.pack(w);
        self.view.pack(w);
    }
}

impl Unpack for ViewId {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let era = Era::unpack(c)?;
        let view = View::unpack(c)?;
        Ok(ViewId { era, view })
    }
}

// ---------------------------------------------------------------------------
// Tag
// ---------------------------------------------------------------------------

/// The message-kind discriminant, first field of every [`Header`].
///
/// `#[repr(u32)]` with an explicit discriminant on every variant, so reordering this
/// source cannot renumber the wire. The numbering is a compatibility surface: a peer
/// compiled from a different revision of this file must agree with it, and "whatever the
/// compiler assigned" is not an agreement.
///
/// `0` is not a tag. It is reserved so that an all-zero buffer — a zeroed page, an
/// unwritten scratch buffer, a datagram padded by a transport — decodes as
/// [`Malformed::UnknownTag`] rather than as a valid message. A codec in which the
/// absence of a message is a message cannot report a framing bug.
///
/// Discriminants `1` and `13` are retired: they belonged to the client-datagram tags,
/// which left the wire when the client boundary became a host concern (§11.1, B2).
/// They stay reserved — reassigning them would collide with any deployment still
/// carrying the old numbering on a wire.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tag {
    /// The primary's proposal of an operation at a slot (§6).
    Prepare = 2,
    /// A replica's acceptance of a `Prepare` (§6).
    PrepareOk = 3,
    /// A commit-frontier advance carrying no new operation (§6).
    Commit = 4,
    /// The ordinary view-change fence: a recipient stops accepting `Prepare` in the old
    /// view (§9.1). Contrast [`Tag::PlannedViewChange`].
    StartViewChange = 5,
    /// A replica's state evidence for the designated new primary (§9.1).
    ///
    /// Whether the evidence is ordinary or planned is a field of the **body**, not of
    /// this tag: the new primary needs to know which kind of quorum it is completing,
    /// but every recipient handles the message the same way, so the distinction does not
    /// carry a dispatch decision the way [`Tag::PlannedViewChange`] does. The
    /// view-change and overlap-mode paths own that field.
    DoViewChange = 6,
    /// The new primary installing the selected history (§9.1, §13.1).
    StartView = 7,
    /// Solicitation of planned view-change evidence during overlap mode (§8.7.7 step 4).
    ///
    /// # Why a distinct tag and not a flag on `StartViewChange`
    ///
    /// The two messages differ in whether they fence the recipient, and that difference
    /// carries the safety argument for the non-stop transition. `StartViewChange` makes
    /// a recipient stop accepting `Prepare` in the current view. `PlannedViewChange`
    /// must not: §8.7.7 has `L` continue committing client operations through `qII`
    /// under view `v` while it collects planned evidence from `qI - {L}`, and step 5 has
    /// each recipient record the planned target *separately* from `current_view`. A
    /// recipient that fenced on it would stall the very stream the non-stop transition
    /// exists to preserve, and it is also sent to a different set of peers — `qI - {L}`
    /// and never `qII - {L}`.
    ///
    /// A shared tag with a boolean would make "is this a fence" a runtime property of a
    /// field rather than of the message type. Every handler would then have to branch
    /// correctly on that field, and a handler that forgot to would silently either fence
    /// when it must not or fail to fence when it must — the first stalls, the second
    /// diverges. With distinct tags the dispatch *is* the check, and a handler cannot
    /// omit it because it never sees the other message.
    PlannedViewChange = 8,
    /// A restarting replica soliciting state; the nonce is the host tick (§10, S4).
    Recovery = 9,
    /// A reply to `Recovery`, echoing the nonce (§10).
    RecoveryResponse = 10,
    /// A request for a history range the requester lacks (§4, §13.1).
    GetState = 11,
    /// A history range in reply to `GetState`. Sizing is the host's (W5).
    NewState = 12,
}

impl Tag {
    /// The wire discriminant.
    ///
    /// A `match` rather than a cast. `self as u32` would keep compiling after a variant
    /// was renumbered by accident; an exhaustive `match` states the numbering in one
    /// place and makes the compiler check that every variant has one.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        match self {
            Tag::Prepare => 2,
            Tag::PrepareOk => 3,
            Tag::Commit => 4,
            Tag::StartViewChange => 5,
            Tag::DoViewChange => 6,
            Tag::StartView => 7,
            Tag::PlannedViewChange => 8,
            Tag::Recovery => 9,
            Tag::RecoveryResponse => 10,
            Tag::GetState => 11,
            Tag::NewState => 12,
        }
    }

    /// The tag a discriminant names, or `None` if it names none.
    ///
    /// `None` for `0`, which is reserved, and for everything above the table. An unknown
    /// tag from a newer peer is refused rather than guessed at: a message whose meaning
    /// is unknown cannot be safely ignored by a replica that may be the only one holding
    /// a committed operation, so the refusal has to be visible to the host that decides
    /// what to do about a version skew.
    #[must_use]
    pub fn from_u32(value: u32) -> Option<Tag> {
        match value {
            2 => Some(Tag::Prepare),
            3 => Some(Tag::PrepareOk),
            4 => Some(Tag::Commit),
            5 => Some(Tag::StartViewChange),
            6 => Some(Tag::DoViewChange),
            7 => Some(Tag::StartView),
            8 => Some(Tag::PlannedViewChange),
            9 => Some(Tag::Recovery),
            10 => Some(Tag::RecoveryResponse),
            11 => Some(Tag::GetState),
            12 => Some(Tag::NewState),
            _ => None,
        }
    }
}

impl Pack for Tag {
    fn packed_len(&self) -> usize {
        size_of::<u32>()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        w.u32(self.as_u32());
    }
}

impl Unpack for Tag {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let raw = c.u32()?;
        Tag::from_u32(raw).ok_or(UnpackError::Malformed(Malformed::UnknownTag(raw)))
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// The 20-byte prefix of every protocol datagram.
///
/// Encoded big-endian as `(tag: u32, era: u32, view: u32, slot: u64)` — decision W1 and
/// Amendment A1, superseding §8.7.3's packed `view = (era << k) | index`.
///
/// # Why era is a header field rather than derived
///
/// Overlap mode (§8.7.7) has the primary address era `e` and era `e+1` **in the same
/// instant**: step 2 keeps streaming client operations to `qII` under view `v` while
/// step 4 solicits planned evidence from `qI - {L}` for a view `v'` whose era is `e+1`.
/// The era authorizing a message is therefore a transport-visible routing fact about
/// that message, not an attribute of the sender's current state — and it is not
/// recoverable from a view number without the configuration history that the packed
/// encoding presumed. §8.7.3's own relation `era(view) <= era(slot) <= era(view) + 1`
/// makes the point: under the packed scheme a host wanting to route or shed by era would
/// have to decode a protocol number to make a transport decision.
///
/// Making era its own field also discharges §8.7.3's checked-encoding requirement and
/// its prohibition on wraparound by construction: two independent `u32` fields cannot
/// alias, so there is no packing to validate and no `k` fixed at genesis to outlive.
///
/// # Why `slot` is in the header
///
/// Not every message names a slot in the protocol sense — `StartViewChange` carries no
/// operation. It is in the header regardless, because a fixed-width header gives every
/// body a fixed offset, which is the same argument W4 makes for fixed-width integers: a
/// size should be a sum, not a parse. What the field *means* is fixed per tag by
/// [`crate::invariant::header_slot_role`]: an operation position, a frontier, or —
/// for messages that speak about no slot at all — the sentinel `Slot(0)`, the one
/// value that can never be confused with a real position, because the first position
/// of a legitimate history is `Void` at slot 1 (§8.7.2). The wire layer stays
/// agnostic: it encodes 20 bytes for every tag and asks no questions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// Which message follows.
    pub tag: Tag,
    /// The era and view authorising the message (§8.7.1, §1.2).
    pub view: ViewId,
    /// Operation position, frontier, or the sentinel `Slot(0)`, per
    /// [`crate::invariant::header_slot_role`].
    pub slot: Slot,
}

impl Header {
    /// Encoded size, fixed at 20 bytes by decision W1.
    ///
    /// A named constant as well as a [`Pack::packed_len`] result, because a host framing
    /// a datagram needs the header size before it has a header to ask. The two are
    /// asserted equal at compile time below and again over arbitrary values in
    /// `tests/wire_contract.rs`, so they cannot drift.
    pub const LEN: usize = 20;
}

// `Header::LEN` against the widths of its fields, at compile time: tag 4, era 4, view 4,
// slot 8. A field added or widened without updating `LEN` becomes a build failure here
// rather than a wire-format disagreement discovered by a peer.
const _: () = assert!(Header::LEN == size_of::<u32>() * 3 + size_of::<u64>());

impl Pack for Header {
    fn packed_len(&self) -> usize {
        self.tag.packed_len() + self.view.packed_len() + self.slot.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.tag.pack(w);
        self.view.pack(w);
        self.slot.pack(w);
    }
}

impl Unpack for Header {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        // Tag first, so an all-zero buffer fails as `UnknownTag(0)` before any other
        // field is interpreted, and so a host logging a rejected datagram has the kind
        // in hand rather than only a length.
        let tag = Tag::unpack(c)?;
        let view = ViewId::unpack(c)?;
        let slot = Slot::unpack(c)?;
        Ok(Header { tag, view, slot })
    }
}

// ---------------------------------------------------------------------------
// JSON debug bridge
// ---------------------------------------------------------------------------

/// Renders a value decoded from the **binary** wire as JSON.
///
/// This is the mechanism that lets a debug or simulation harness stress the binary path
/// while reading JSON: nodes append their raw binary in/out stream to a file and a
/// post-mortem pass renders it. A JSON-native debug transport would exercise a codec
/// that never ships and prove nothing about the one that does, which is the mistake W3
/// was written to undo.
///
/// # Errors
///
/// As [`Unpack::unpack_from`]. A serialization failure is reported as
/// [`Malformed::OutOfDomain`] rather than panicking; it is unreachable for the types in
/// this crate, whose derived `Serialize` impls cover scalars and fieldless enums only,
/// and stating it as an error arm keeps the bridge total like the rest of the module.
#[cfg(feature = "maelstrom")]
pub fn unpack_to_json<T>(bytes: &[u8]) -> Result<String, UnpackError>
where
    T: Unpack + serde::Serialize,
{
    let value = T::unpack_from(bytes)?;
    serde_json::to_string(&value).map_err(|_| UnpackError::Malformed(Malformed::OutOfDomain))
}
