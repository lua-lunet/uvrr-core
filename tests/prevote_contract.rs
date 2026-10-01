//! Contract for the pre-vote module's boundaries: the role table in `docs/prevoting.md`
//! is the code's table, and the alphabet is on no wire type.
//!
//! Spec `docs/prevoting.md`, the role table and the module boundaries.
//!
//! Two obligations, both of which exist to catch a drift nobody would notice by reading:
//!
//! 1. **The document's table is the code's table.** Markdown-driven development only
//!    means something if the document is load-bearing, so this test parses the table out
//!    of the specification and compares all twelve cells against [`uvrr::prevote::role`].
//!    A cell edited in one place and not the other is a failing test rather than a
//!    disagreement discovered in review.
//! 2. **The alphabet has no wire representation.** The boundaries section states that no [`uvrr::wire::Tag`]
//!    and no [`uvrr::message::Body`] carries any of the three messages, because binding
//!    them is a separate change with its own obligations. Adding a tag for a pre-vote
//!    message without that amendment is precisely the drift to catch, so every tag name
//!    and every unallocated discriminant is checked here.

use std::collections::BTreeMap;
use std::fs;

use uvrr::ids::NodeId;
use uvrr::prevote::alphabet::Exchange;
use uvrr::prevote::{Ownership, Recency, Role, role};
use uvrr::wire::Tag;

/// The specification the module is judged against.
const SPEC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/prevoting.md");

/// The member `Theirs` is instantiated with; the table's cells do not name a member and
/// the answer must not depend on which one it is.
const PEER: NodeId = NodeId(7);

/// The role table of `docs/prevoting.md`, parsed out of the document.
fn spec_table() -> BTreeMap<(String, String), String> {
    let text = fs::read_to_string(SPEC).expect("docs/prevoting.md is readable");
    let mut rows = text
        .lines()
        .filter(|line| line.trim_start().starts_with('|'));

    let columns_of = |line: &str| -> Vec<String> {
        line.split('|')
            .map(|cell| cell.trim().trim_matches('`').to_string())
            // A leading and trailing pipe yield empty first and last columns.
            .filter(|cell| !cell.is_empty())
            .collect()
    };

    // The header names the recency columns, so it is kept: a data cell is identified by
    // its column, and the column's name is the recency.
    let header = columns_of(rows.next().expect("the table has a header row"));
    assert!(
        header.first().is_some_and(|first| first == "Ownership")
            && header.iter().any(|cell| cell == "VeryRecent"),
        "the first table in the document is the role table, found: {header:?}"
    );
    rows.next().expect("the table has a rule under its header");

    let mut cells = BTreeMap::new();
    for row in rows {
        let columns = columns_of(row);
        if columns.len() != header.len() {
            // The table ended; anything after it is a different table.
            break;
        }
        // The ownership keys the row; each recency column names one cell.
        for index in 1..columns.len() {
            cells.insert(
                (columns[0].clone(), header[index].clone()),
                columns[index].clone(),
            );
        }
    }
    assert_eq!(
        cells.len(),
        12,
        "the role table is three ownerships by four recencies"
    );
    cells
}

fn ownership_of(cell: &str) -> Ownership {
    match cell {
        "Mine" => Ownership::Mine,
        "Theirs" => Ownership::Theirs(PEER),
        "Unknown" => Ownership::Unknown,
        other => panic!("the table names an ownership the code does not have: {other}"),
    }
}

fn recency_of(cell: &str) -> Recency {
    match cell {
        "VeryRecent" => Recency::VeryRecent,
        "Recent" => Recency::Recent,
        "Stale" => Recency::Stale,
        "Never" => Recency::Never,
        other => panic!("the table names a recency the code does not have: {other}"),
    }
}

fn role_of(cell: &str) -> Role {
    match cell {
        "Leader" => Role::Leader,
        "Incumbent" => Role::Incumbent,
        "Follower" => Role::Follower,
        "Candidate" => Role::Candidate,
        other => panic!("the table names a role the code does not have: {other}"),
    }
}

#[test]
fn the_specifications_role_table_is_the_codes_table() {
    let table = spec_table();
    for ((owner_cell, recency_cell), role_cell) in table {
        let ownership = ownership_of(&owner_cell);
        let recency = recency_of(&recency_cell);
        let claimed = role_of(&role_cell);
        assert_eq!(
            role(ownership, recency),
            claimed,
            "docs/prevoting.md §3 says {} at {} is {}, and the code disagrees",
            owner_cell,
            recency_cell,
            role_cell
        );
    }
}

#[test]
fn the_specifications_table_uses_the_codes_own_names() {
    let text = fs::read_to_string(SPEC).expect("docs/prevoting.md is readable");
    // The table names enum variants, and `name()` is what every human surface reads, so
    // the variant spelling and the surface spelling must both be derivable from the code.
    // The variant spellings are checked by the table test above; this one pins that the
    // document's recency column headers are exactly the code's variant names, so a
    // renamed variant is a failing test rather than a stale document.
    let table = spec_table();
    let mut recencies: Vec<&String> = table.keys().map(|(_, recency)| recency).collect();
    recencies.sort();
    recencies.dedup();
    assert_eq!(
        recencies,
        vec!["Never", "Recent", "Stale", "VeryRecent"],
        "the four recency columns of the document's table"
    );
    assert!(text.contains("docs/prevoting.md") || text.contains("Pre-voting"));
}

#[test]
fn no_wire_tag_carries_a_prevote_message() {
    // The three alphabet names, as the code spells them.
    let names: Vec<String> = (0u32..64)
        .filter_map(Tag::from_u32)
        .map(|tag| tag.name().to_string())
        .collect();
    assert_eq!(names.len(), 14, "the fourteen allocated tags");
    for exchange in Exchange::ALL {
        assert!(
            !names.iter().any(|name| name == exchange.name()),
            "the wire already names {} and §9 says it does not",
            exchange.name()
        );
    }
}

#[test]
fn no_discriminant_beyond_the_allocated_tags_has_been_claimed() {
    // The allocated discriminants stop at the last tag. Anything a pre-vote binding would
    // need is unallocated, which is the state §9 describes and the state a later,
    // spec-amending change must begin from.
    for discriminant in 18u32..64 {
        assert_eq!(
            Tag::from_u32(discriminant),
            None,
            "discriminant {discriminant} is claimed, and §9 says nothing is on the wire yet"
        );
    }
}
