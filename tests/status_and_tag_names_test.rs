//! The human surfaces of the process-control state and the wire tags:
//! every status and every tag carries a name stated next to the numbering
//! it names, so no log, trace, or diagnostic dump ever asks a human to
//! memorise an integer. The numeric encodings themselves are pinned here —
//! a rename or a renumber that moves a word off the wire is caught.

use vrr::progress::Status;
use vrr::wire::Tag;

/// Every status word names itself, and the numbering is unchanged: the
/// snapshot word and the wire stay numeric.
#[test]
fn every_status_names_itself_and_the_words_are_unchanged() {
    let cases: [(Status, u32, &str); 5] = [
        (Status::Normal, 0, "normal"),
        (Status::ViewChange, 1, "view_change"),
        (Status::Restarting, 2, "restarting"),
        (Status::Replaying, 3, "replaying"),
        (Status::Joining, 4, "joining"),
    ];
    for (status, word, name) in cases {
        assert_eq!(status.to_word(), word);
        assert_eq!(Status::from_word(word), Some(status));
        assert_eq!(status.name(), name);
    }
    assert_eq!(Status::from_word(5), None, "no word above the table");
}

/// Every tag discriminant names itself, and the numbering is unchanged:
/// the wire encoding stays numeric.
#[test]
fn every_tag_names_itself_and_the_discriminants_are_unchanged() {
    let cases: [(Tag, u32, &str); 14] = [
        (Tag::Prepare, 2, "prepare"),
        (Tag::PrepareOk, 3, "prepare_ok"),
        (Tag::Commit, 4, "commit"),
        (Tag::StartViewChange, 5, "start_view_change"),
        (Tag::DoViewChange, 6, "do_view_change"),
        (Tag::StartView, 7, "start_view"),
        (Tag::PlannedViewChange, 8, "planned_view_change"),
        (Tag::GetState, 9, "get_state"),
        (Tag::NewState, 10, "new_state"),
        (Tag::Reincarnation, 13, "reincarnation"),
        (Tag::Fuse, 14, "fuse"),
        (Tag::FuseOk, 15, "fuse_ok"),
        (Tag::CommitBatch, 16, "commit_batch"),
        (Tag::GossipRequest, 17, "gossip_request"),
    ];
    for (tag, discriminant, name) in cases {
        assert_eq!(tag.as_u32(), discriminant);
        assert_eq!(Tag::from_u32(discriminant), Some(tag));
        assert_eq!(tag.name(), name);
    }
    assert_eq!(Tag::from_u32(0), None, "zero is reserved");
    assert_eq!(Tag::from_u32(1), None);
    assert_eq!(Tag::from_u32(11), None, "the retired tags stay retired");
    assert_eq!(Tag::from_u32(12), None, "the retired tags stay retired");
    assert_eq!(Tag::from_u32(18), None, "no discriminant above the table");
}
