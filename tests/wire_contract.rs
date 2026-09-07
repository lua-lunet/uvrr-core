//! Contract for `vrr::wire` — the normative binary codec substrate.
//!
//! Spec §11 (opaque client payloads), §13.1 (bounded view-change suffix), and decisions
//! W1 (explicit `ViewId`, 20-byte big-endian header), W3 (own binary codec, serde
//! optional), W5 (no datagram sizing in the core), W4 (fixed-width big-endian, no
//! varints).
//!
//! This file gates the codec substrate only. It knows about no message body, because
//! `Prepare` carries a log entry and `StartView` carries configuration evidence, and
//! neither exists yet. Later message bodies add their own round-trip tests through the traits
//! pinned here.
//!
//! Five properties, each of which the rest of the crate is entitled to assume without
//! re-deriving it:
//!
//! 1. **The wire is pinned by golden vectors.** `Header` is exactly 20 bytes in a
//!    stated order, and `OperationId` is big-endian. A golden vector is the only test that
//!    catches a symmetric mistake — an encoder and decoder that agree with each other
//!    and disagree with the specification round-trip perfectly.
//! 2. **`packed_len()` is normative, not advisory (W3).** It equals the byte count
//!    `pack_into` writes, for every value. The §13.1 suffix budget is a sum only if
//!    this holds; if it were an upper bound the budget would be a search. This is why
//!    W4 forbids varints, and it is a test rather than a comment.
//! 3. **Totality (W5).** `UnpackCursor` returns `Ok` or `Err` on every input, including
//!    adversarial input, in release as well as debug. `Incomplete` and `Malformed` are
//!    distinguished so a host reassembling a fragmented datagram can tell "ask again"
//!    from "drop the peer" without guessing.
//! 4. **No partial write.** A failed `pack_into` leaves the caller's buffer untouched,
//!    so a host may reuse one scratch buffer across attempts.
//! 5. **The absence of a constant is enforced (W5).** No source file names a datagram
//!    size. A grep-as-test is crude and is exactly right: it makes an absence into a
//!    property the build checks.
//!
//! Groups 5, 7 and 10 are exhaustive loops rather than samplers — the domains are tiny
//! and total coverage is strictly stronger than any number of random draws.

use proptest::prelude::*;
use vrr::ids::{Era, NodeId, OperationId, Slot, Tick, View, ViewId};
use vrr::wire::{Header, Malformed, Pack, PackError, Tag, Unpack, UnpackCursor, UnpackError};

/// Every `Tag`, in discriminant order. Used by the round-trip and exhaustiveness
/// groups. Kept as an explicit list rather than derived from a `Tag::ALL` constant so
/// that the test agrees with the brief's table independently of the implementation.
const ALL_TAGS: [Tag; 9] = [
    Tag::Prepare,
    Tag::PrepareOk,
    Tag::Commit,
    Tag::StartViewChange,
    Tag::DoViewChange,
    Tag::StartView,
    Tag::PlannedViewChange,
    Tag::GetState,
    Tag::NewState,
];

/// Encodes `value` into a fresh `Vec` sized by `packed_len()` and asserts the write
/// filled it exactly. Test-side helper only: the crate API is allocation-free by
/// design, and a test is entitled to allocate.
fn encode<T: Pack>(value: &T) -> Vec<u8> {
    let needed = value.packed_len();
    let mut buf = vec![0u8; needed];
    let written = value
        .pack_into(&mut buf)
        .expect("a buffer of exactly packed_len() bytes must suffice");
    assert_eq!(
        written, needed,
        "packed_len() is normative (W3): it must equal the bytes written"
    );
    buf
}

// ---------------------------------------------------------------------------
// 1. `Header` golden vector
// ---------------------------------------------------------------------------

/// The 20-byte header of decision W1 is `(tag: u32, era: u32, view: u32, slot: u64)`,
/// big-endian, in that order and with no padding.
///
/// Byte-for-byte rather than a round trip: a round trip passes when the encoder and the
/// decoder share a mistake. This is the assertion that a host writing a decoder in C or
/// LuaJIT from the specification will agree with.
#[test]
fn header_golden_vector() {
    assert_eq!(Header::LEN, 20, "decision W1 fixes the header at 20 bytes");

    let header = Header {
        tag: Tag::Prepare,
        view: ViewId {
            era: Era(0x0102_0304),
            view: View(0x0506_0708),
        },
        slot: Slot(0x090a_0b0c_0d0e_0f10),
    };

    let bytes = encode(&header);

    assert_eq!(
        bytes,
        vec![
            // tag: Prepare = 2
            0x00, 0x00, 0x00, 0x02, //
            // era
            0x01, 0x02, 0x03, 0x04, //
            // view
            0x05, 0x06, 0x07, 0x08, //
            // slot
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        ],
        "header field order and endianness are normative (W1)"
    );
    assert_eq!(bytes.len(), Header::LEN);
    assert_eq!(header.packed_len(), Header::LEN);
}

// ---------------------------------------------------------------------------
// 2. `OperationId` golden vector
// ---------------------------------------------------------------------------

/// An `OperationId` is 16 bytes, most-significant word first, each word
/// big-endian — like every other integer on this wire (W4). The identity is
/// opaque to the core (§11.1, B2), but its wire shape is pinned by a vector
/// so two hosts cannot disagree about the byte order.
#[test]
fn operation_id_golden_vector_is_big_endian() {
    let id = OperationId {
        msb: 0x0011_2233_4455_6677,
        lsb: 0x8899_aabb_ccdd_eeff,
    };
    assert_eq!(
        encode(&id),
        vec![
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ],
        "OperationId is 16 bytes, msb first, both words big-endian (W4)"
    );
    assert_eq!(id.packed_len(), 16);

    // The most significant byte of the least significant word trails.
    let one = OperationId { msb: 0, lsb: 1 };
    let mut expect = vec![0u8; 16];
    expect[15] = 0x01;
    assert_eq!(encode(&one), expect);

    // The most significant word leads.
    let high = OperationId { msb: 1, lsb: 0 };
    let mut expect = vec![0u8; 16];
    expect[7] = 0x01;
    assert_eq!(encode(&high), expect);
}

// ---------------------------------------------------------------------------
// 3. `packed_len` is exact
// ---------------------------------------------------------------------------

/// A generator for arbitrary `Header` values, reused by groups 3 and 4.
fn any_header() -> impl Strategy<Value = Header> {
    (
        0usize..ALL_TAGS.len(),
        any::<u32>(),
        any::<u32>(),
        any::<u64>(),
    )
        .prop_map(|(tag_index, era, view, slot)| Header {
            tag: ALL_TAGS[tag_index],
            view: ViewId {
                era: Era(era),
                view: View(view),
            },
            slot: Slot(slot),
        })
}

proptest! {
    /// W3's claim in mechanical form: `packed_len()` is the exact encoded size, so the
    /// §13.1 suffix budget is a sum rather than a search. `encode` asserts the equality
    /// on every call; these cases exercise it across the primitive domain.
    #[test]
    fn packed_len_is_exact(
        a in any::<u8>(),
        b in any::<u16>(),
        c in any::<u32>(),
        d in any::<u64>(),
        e in any::<u128>(),
        f in any::<bool>(),
        header in any_header(),
        payload in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        prop_assert_eq!(encode(&a).len(), 1);
        prop_assert_eq!(encode(&b).len(), 2);
        prop_assert_eq!(encode(&c).len(), 4);
        prop_assert_eq!(encode(&d).len(), 8);
        prop_assert_eq!(encode(&e).len(), 16);
        prop_assert_eq!(encode(&f).len(), 1);
        prop_assert_eq!(encode(&header).len(), Header::LEN);

        // Opaque bytes: a u32 big-endian length prefix and then the payload (§11).
        prop_assert_eq!(vrr::wire::opaque_packed_len(&payload), 4 + payload.len());
        let mut buf = vec![0u8; vrr::wire::opaque_packed_len(&payload)];
        let written = vrr::wire::pack_opaque_into(&payload, &mut buf)
            .expect("exactly-sized buffer must suffice");
        prop_assert_eq!(written, 4 + payload.len());
    }
}

// ---------------------------------------------------------------------------
// 4. Round trip
// ---------------------------------------------------------------------------

/// `unpack_from(encode(x)) == x`, for every primitive, every `ids` newtype, `ViewId`,
/// `Tag`, and `Header`. `unpack_from` rejects trailing bytes, so this simultaneously
/// asserts that each encoding consumes exactly its own bytes.
fn round_trip<T>(value: T)
where
    T: Pack + Unpack + PartialEq + core::fmt::Debug,
{
    let bytes = encode(&value);
    let decoded = T::unpack_from(&bytes).expect("a self-produced encoding must decode");
    assert_eq!(decoded, value, "round trip must be the identity");
}

proptest! {
    #[test]
    fn round_trip_primitives_and_ids(
        a in any::<u8>(),
        b in any::<u16>(),
        c in any::<u32>(),
        d in any::<u64>(),
        e in any::<u128>(),
        f in any::<bool>(),
        header in any_header(),
    ) {
        round_trip(a);
        round_trip(b);
        round_trip(c);
        round_trip(d);
        round_trip(e);
        round_trip(f);

        round_trip(NodeId(c));
        round_trip(Era(c));
        round_trip(View(c));
        round_trip(Slot(d));
        round_trip(Tick(d));
        round_trip(OperationId { msb: d, lsb: d });
        round_trip(ViewId { era: Era(c), view: View(c) });
        round_trip(header);
    }
}

proptest! {
    /// Opaque payloads round-trip by borrowing out of the cursor rather than copying, so
    /// a host client payload crosses the codec without an allocation (§11: the core never
    /// inspects it).
    #[test]
    fn round_trip_opaque(payload in prop::collection::vec(any::<u8>(), 0..1024)) {
        let mut buf = vec![0u8; vrr::wire::opaque_packed_len(&payload)];
        vrr::wire::pack_opaque_into(&payload, &mut buf).expect("exactly sized");
        let mut cursor = UnpackCursor::new(&buf);
        let seen = cursor.opaque().expect("self-produced encoding must decode");
        prop_assert_eq!(seen, payload.as_slice());
        prop_assert!(cursor.finish().is_ok());
    }
}

/// Every `Tag` round-trips through its `u32` discriminant.
#[test]
fn round_trip_every_tag() {
    for tag in ALL_TAGS {
        round_trip(tag);
    }
}

// ---------------------------------------------------------------------------
// 5. Truncation is `Incomplete`, never `Malformed`, never a panic
// ---------------------------------------------------------------------------

/// Exhaustive over every proper prefix of a valid encoding.
///
/// This is the W5 distinction in its most load-bearing form: a host reassembling a
/// fragmented datagram must be able to keep asking for more bytes, and must never be
/// told the message is garbage merely because it has not all arrived. `needed` is the
/// total byte count required to pass the point that failed, so it is strictly greater
/// than the prefix length and a caller can use it directly as a reservation size.
#[test]
fn every_prefix_of_a_header_is_incomplete() {
    let header = Header {
        tag: Tag::StartView,
        view: ViewId {
            era: Era(7),
            view: View(9),
        },
        slot: Slot(u64::MAX),
    };
    let bytes = encode(&header);

    for cut in 0..bytes.len() {
        let prefix = &bytes[..cut];
        match Header::unpack_from(prefix) {
            Err(UnpackError::Incomplete { needed }) => {
                assert!(
                    needed > prefix.len(),
                    "needed ({needed}) must exceed the prefix length ({cut}); it is a \
                     total requirement, not a shortfall"
                );
                assert!(
                    needed <= bytes.len(),
                    "needed ({needed}) must not exceed the true encoded length"
                );
            }
            other => panic!("prefix of length {cut} must be Incomplete, got {other:?}"),
        }
    }
}

/// The same property for the opaque-bytes primitive, whose length is data-dependent:
/// truncation inside the four-byte prefix and truncation inside the payload must both
/// report `Incomplete`.
#[test]
fn every_prefix_of_opaque_bytes_is_incomplete() {
    for payload_len in [0usize, 1, 2, 17, 255, 256, 300] {
        let payload: Vec<u8> = (0..payload_len)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let mut bytes = vec![0u8; vrr::wire::opaque_packed_len(&payload)];
        vrr::wire::pack_opaque_into(&payload, &mut bytes).expect("exactly sized");

        for cut in 0..bytes.len() {
            let prefix = &bytes[..cut];
            let mut cursor = UnpackCursor::new(prefix);
            match cursor.opaque() {
                Err(UnpackError::Incomplete { needed }) => {
                    assert!(
                        needed > prefix.len(),
                        "payload_len {payload_len}, cut {cut}: needed {needed} must exceed \
                         the prefix length"
                    );
                }
                other => {
                    panic!("payload_len {payload_len}, cut {cut} must be Incomplete, got {other:?}")
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Adversarial input is total
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// Totality is the headline property of this module. Arbitrary bytes must produce
    /// `Ok` or `Err`, never a panic — in release as well as debug, which is why
    /// `cargo test --release` is a required gate for this file. An overflow that only
    /// `debug_assert` catches is a production bug that the test suite hid.
    #[test]
    fn arbitrary_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        let _ = Header::unpack_from(&bytes);
        let _ = Tag::unpack_from(&bytes);
        let _ = ViewId::unpack_from(&bytes);
        let _ = OperationId::unpack_from(&bytes);
        let _ = bool::unpack_from(&bytes);

        // Drive the cursor directly, in a sequence that mixes widths, so a length
        // prefix read at an arbitrary offset is exercised too.
        let mut cursor = UnpackCursor::new(&bytes);
        let _ = cursor.u8();
        let _ = cursor.u16();
        let _ = cursor.bool();
        let _ = cursor.opaque();
        let _ = cursor.u128();
        let _ = cursor.u64();
        let _ = cursor.bytes16();
        let _ = cursor.u32();
        let _ = cursor.finish();
    }
}

// ---------------------------------------------------------------------------
// 7. `Malformed` cases
// ---------------------------------------------------------------------------

/// Zero is reserved, so an all-zero buffer is not a message. Without the reservation a
/// zeroed page, a zero-filled scratch buffer, or a padded datagram would decode as the
/// lowest-numbered tag, and the codec would have no way to tell a message from an absence.
#[test]
fn tag_zero_is_reserved() {
    let bytes = [0u8; 4];
    assert_eq!(
        Tag::unpack_from(&bytes),
        Err(UnpackError::Malformed(Malformed::UnknownTag(0)))
    );

    let zeroed = [0u8; Header::LEN];
    assert_eq!(
        Header::unpack_from(&zeroed),
        Err(UnpackError::Malformed(Malformed::UnknownTag(0))),
        "an all-zero header must not be a valid message"
    );
}

/// Every discriminant outside `1..=13` is an unknown tag, reported with the offending
/// value so a host can log what it dropped.
#[test]
fn unknown_tags_are_rejected() {
    for candidate in [0u32, 14, 15, 100, u32::MAX] {
        let bytes = candidate.to_be_bytes();
        assert_eq!(
            Tag::unpack_from(&bytes),
            Err(UnpackError::Malformed(Malformed::UnknownTag(candidate))),
            "discriminant {candidate} must be unknown"
        );
    }
}

/// `bool` is `0x00` or `0x01` and nothing else. A permissive "non-zero is true" reading
/// would make two distinct byte strings decode to the same message, which breaks any
/// later argument that a message has a canonical encoding.
#[test]
fn non_canonical_bool_is_out_of_domain() {
    assert_eq!(bool::unpack_from(&[0x00]), Ok(false));
    assert_eq!(bool::unpack_from(&[0x01]), Ok(true));
    for byte in 2u8..=255 {
        assert_eq!(
            bool::unpack_from(&[byte]),
            Err(UnpackError::Malformed(Malformed::OutOfDomain)),
            "byte {byte:#04x} is not a bool"
        );
    }
}

/// The `LengthPrefixOverflow` / `Incomplete` boundary.
///
/// Rule: a declared length is `Incomplete` whenever the total byte count it implies is
/// *representable*, and `LengthPrefixOverflow` only when it is not — that is, when
/// `cursor_offset + declared_len` overflows `usize`. A prefix that merely exceeds the
/// bytes currently in hand is unsatisfied, not impossible, because the host may still
/// be reassembling; telling it "malformed" would make it drop a peer over a fragment
/// boundary. A prefix whose implied total cannot be expressed as a `usize` can never be
/// satisfied by any input on this machine, so it is malformed.
///
/// On a 64-bit target a `u32` prefix can never overflow a `usize`, so the overflow arm
/// is unreachable there and the boundary is exercised by the `Incomplete` side; the arm
/// exists because a 32-bit target reaches it and because the check must not be an
/// unchecked add.
#[test]
fn length_prefix_boundary() {
    // Declared 8, supplied 7: unsatisfied, so Incomplete with the total requirement.
    let mut bytes = 8u32.to_be_bytes().to_vec();
    bytes.extend_from_slice(&[0u8; 7]);
    let mut cursor = UnpackCursor::new(&bytes);
    assert_eq!(
        cursor.opaque(),
        Err(UnpackError::Incomplete { needed: 12 }),
        "a prefix larger than the bytes in hand is Incomplete, not Malformed"
    );

    // Declared 8, supplied 8: satisfied exactly.
    let mut bytes = 8u32.to_be_bytes().to_vec();
    bytes.extend_from_slice(&[0xABu8; 8]);
    let mut cursor = UnpackCursor::new(&bytes);
    assert_eq!(cursor.opaque(), Ok(&[0xABu8; 8][..]));
    assert_eq!(cursor.finish(), Ok(()));

    // u32::MAX declared with nothing supplied: still merely unsatisfied on a 64-bit
    // target, and the arithmetic must not overflow while computing `needed`.
    let bytes = u32::MAX.to_be_bytes();
    let mut cursor = UnpackCursor::new(&bytes);
    let overflow_is_reachable = usize::BITS < 36;
    match cursor.opaque() {
        Err(UnpackError::Incomplete { needed }) => {
            assert!(needed > bytes.len());
            assert!(
                !overflow_is_reachable,
                "a target too narrow to express the total must report the overflow"
            );
        }
        Err(UnpackError::Malformed(Malformed::LengthPrefixOverflow)) => {
            // Only reachable where `usize` is too narrow to express the implied total.
            assert!(
                overflow_is_reachable,
                "on a target where the total is representable the prefix is merely \
                 unsatisfied, not impossible"
            );
        }
        other => panic!("expected Incomplete or LengthPrefixOverflow, got {other:?}"),
    }
}

/// A message with trailing garbage is not a message. `unpack_from` calls `finish()`, so
/// the surplus is reported with its count rather than silently ignored — a decoder that
/// ignores a suffix cannot detect a framing bug in the host's reassembly.
#[test]
fn trailing_bytes_are_rejected() {
    let header = Header {
        tag: Tag::Commit,
        view: ViewId::INITIAL,
        slot: Slot::NONE,
    };
    let mut bytes = encode(&header);
    bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF]);

    assert_eq!(
        Header::unpack_from(&bytes),
        Err(UnpackError::Malformed(Malformed::TrailingBytes {
            unread: 3
        }))
    );
}

// ---------------------------------------------------------------------------
// 8. No partial write
// ---------------------------------------------------------------------------

/// `PackWriter::new` performs the single bounds check, so a short buffer is refused
/// before any byte is written. A host reusing a scratch buffer across a
/// budget-exceeded retry relies on this: if a failed attempt could scribble a prefix,
/// the host would have to zero the buffer between attempts, which is exactly the cost
/// W5 is trying to keep out of the core.
#[test]
fn pack_into_makes_no_partial_write() {
    let header = Header {
        tag: Tag::DoViewChange,
        view: ViewId {
            era: Era(1),
            view: View(2),
        },
        slot: Slot(3),
    };
    let needed = header.packed_len();

    for provided in 0..needed {
        let mut buf = vec![0xAAu8; needed];
        let outcome = header.pack_into(&mut buf[..provided]);
        assert_eq!(
            outcome,
            Err(PackError::BufferTooSmall { needed, provided }),
            "a {provided}-byte buffer must be refused with the size required"
        );
        assert!(
            buf.iter().all(|&b| b == 0xAA),
            "a refused pack must not touch the buffer"
        );
    }

    // And the exactly-sized buffer succeeds.
    let mut buf = vec![0xAAu8; needed];
    assert_eq!(header.pack_into(&mut buf), Ok(needed));
    assert_ne!(buf, vec![0xAAu8; needed]);
}

// ---------------------------------------------------------------------------
// 9. W5 mechanical gate
// ---------------------------------------------------------------------------

/// The forbidden names. `MTU` is matched case-sensitively and as an identifier fragment,
/// which is why the scan strips comments first: W5's own rationale has to be allowed to
/// *say* the words it forbids, or the decision could not be documented in the module
/// that implements it.
const FORBIDDEN: [&str; 4] = ["MAX_DATAGRAM", "65507", "1500", "MTU"];

/// Removes `//`-style comments, so the gate reads code and not prose.
///
/// Crude on purpose: it does not understand `//` inside a string literal or a block
/// comment. Both would only cause a false *failure*, never a false pass, and this crate
/// controls its own sources.
fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Makes the absence of a datagram-size constant a property the build enforces.
///
/// W5 says the core owns no packetization decision. That is easy to state and easy to
/// erode: one `const MAX_DATAGRAM` added for a plausible local reason and the core is
/// silently making a transport decision on behalf of a host it has never met. A grep is
/// a blunt instrument and it is the right one, because the property being asserted is
/// the absence of a name rather than the behaviour of a function.
#[test]
fn no_source_file_names_a_datagram_size() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0usize;

    let entries = std::fs::read_dir(&src).expect("src/ must be readable");
    for entry in entries {
        let path = entry.expect("directory entry must be readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source file must be readable");
        let code = code_only(&source);
        for needle in FORBIDDEN {
            assert!(
                !code.contains(needle),
                "{}: contains `{needle}`; the core owns no packetization decision (W5)",
                path.display()
            );
        }
        checked += 1;
    }

    assert!(
        checked >= 10,
        "expected to scan the whole module set, scanned only {checked} files"
    );
}

// ---------------------------------------------------------------------------
// 10. `Tag` exhaustiveness
// ---------------------------------------------------------------------------

/// A wildcard-free `match` over every variant. Adding a fourteenth tag without deciding
/// its discriminant and its round trip is a compile error here, which is the point of
/// the test: the wire numbering is not something a later change may extend by accident.
#[test]
fn tag_match_is_exhaustive_and_discriminants_are_pinned() {
    for tag in ALL_TAGS {
        let discriminant: u32 = match tag {
            Tag::Prepare => 2,
            Tag::PrepareOk => 3,
            Tag::Commit => 4,
            Tag::StartViewChange => 5,
            Tag::DoViewChange => 6,
            Tag::StartView => 7,
            Tag::PlannedViewChange => 8,
            Tag::GetState => 9,
            Tag::NewState => 10,
        };
        assert_eq!(
            tag.as_u32(),
            discriminant,
            "{tag:?} must encode as {discriminant}"
        );
        assert_eq!(
            Tag::from_u32(discriminant),
            Some(tag),
            "discriminant {discriminant} must decode to {tag:?}"
        );
        assert_eq!(encode(&tag), discriminant.to_be_bytes().to_vec());
    }

    // The whole numbering, including both boundaries of the reserved space.
    // Discriminants 1, 11 and 12 belonged to the retired recovery-exchange
    // tags and the retired client-datagram tags (B2: client traffic is a
    // host concern, never a core datagram); they stay reserved — reuse
    // would collide with deployments that still carry the old numbering on
    // a wire.
    for candidate in 2u32..=10 {
        let tag = Tag::from_u32(candidate).expect("2..=10 are all tags");
        assert_eq!(tag.as_u32(), candidate);
    }
    assert_eq!(Tag::from_u32(0), None, "0 is reserved, not a tag");
    assert_eq!(Tag::from_u32(1), None, "1 is retired, not a tag");
    assert_eq!(Tag::from_u32(11), None, "11 is retired, not a tag");
    assert_eq!(Tag::from_u32(12), None, "12 is retired, not a tag");
    assert_eq!(Tag::from_u32(13), None, "13 is not yet a tag");

    // `PlannedViewChange` is a distinct tag rather than a flag on `StartViewChange`
    // (§8.7.7 step 4). Distinct discriminants are the mechanical expression of that.
    assert_ne!(
        Tag::PlannedViewChange.as_u32(),
        Tag::StartViewChange.as_u32()
    );
}

// ---------------------------------------------------------------------------
// 11. serde round trip
// ---------------------------------------------------------------------------

/// Serde is for hosts that want their own encoding and for the debug bridge; it is never
/// on the datagram path (W3). The round trip is asserted so that the two encoders cannot
/// drift into disagreeing about which fields exist or in what order.
///
/// The format is `serde_json`, a `[dev-dependencies]` entry. It is deliberately *not*
/// reachable from `[dependencies]` without the `maelstrom` feature: what W3 protects is a
/// consumer's dependency set, and a dev-dependency is build-time tooling for `cargo test`
/// that no downstream crate resolves. Asserting against a real format keeps this test
/// about our derives; a hand-rolled `Serializer` would test serde.
#[cfg(feature = "serde")]
#[test]
fn serde_round_trip_header() {
    let header = Header {
        tag: Tag::PlannedViewChange,
        view: ViewId {
            era: Era(4),
            view: View(11),
        },
        slot: Slot(1234),
    };

    let json = serde_json::to_string(&header).expect("Header must serialize");

    // The exact string, not a parsed `Value`: serde_json emits struct fields in
    // declaration order, so the literal pins the field set *and* the order, whereas a
    // `Value` comparison would sort the keys and lose the ordering assertion. `Tag` is a
    // fieldless enum, so it travels as its variant name; `ViewId` nests rather than
    // flattening, because it is a struct field and not `#[serde(flatten)]`.
    assert_eq!(
        json, r#"{"tag":"PlannedViewChange","view":{"era":4,"view":11},"slot":1234}"#,
        "serde must visit tag, view (era then view), slot in the W1 header order"
    );

    let back: Header = serde_json::from_str(&json).expect("Header must deserialize");
    assert_eq!(back, header, "round trip must be the identity");
}

/// The `ids` newtypes carry the same feature-gated derives, so a host imposing its own
/// encoding gets the whole identity surface rather than only the header.
#[cfg(feature = "serde")]
#[test]
fn serde_round_trip_ids() {
    fn assert_round_trip<T>(value: T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
    {
        let json = serde_json::to_string(&value).expect("must serialize");
        let back: T = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(back, value, "round trip must be the identity: {json}");
    }

    assert_round_trip(NodeId(7));
    assert_round_trip(Era(3));
    assert_round_trip(View(9));
    assert_round_trip(Slot(u64::MAX));
    assert_round_trip(Tick(1_000_000));
    assert_round_trip(OperationId {
        msb: u64::MAX,
        lsb: u64::MAX,
    });
    assert_round_trip(OperationId { msb: 0, lsb: 42 });
    assert_round_trip(ViewId {
        era: Era(1),
        view: View(2),
    });

    for tag in ALL_TAGS {
        assert_round_trip(tag);
    }
}

/// The binary-to-JSON debug bridge renders a datagram captured off the *binary* path,
/// which is the whole point: a JSON-native debug path would exercise the wrong codec
/// and prove nothing about the encoding that shipped.
#[cfg(feature = "maelstrom")]
#[test]
fn unpack_to_json_renders_a_binary_header() {
    let header = Header {
        tag: Tag::PlannedViewChange,
        view: ViewId {
            era: Era(2),
            view: View(5),
        },
        slot: Slot(77),
    };
    let bytes = encode(&header);

    let rendered = vrr::wire::unpack_to_json::<Header>(&bytes).expect("must render");
    assert!(
        rendered.contains("PlannedViewChange"),
        "rendered: {rendered}"
    );
    assert!(rendered.contains("77"), "rendered: {rendered}");

    assert!(
        vrr::wire::unpack_to_json::<Header>(&[0u8; 4]).is_err(),
        "a short buffer must not render"
    );
}
