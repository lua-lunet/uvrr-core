mod support;

use support::{ReplicaSnapshot, assert_replica_unchanged, message, node, receive, state};
use vrr::vrr::{Body, Input};

#[test]
fn recovering_replica_refuses_future_do_view_change_without_mutation() {
    let mut replica = node(4, 1);
    replica.step(Input::Recover { nonce: 7 });
    let before = ReplicaSnapshot::capture(&replica);

    assert!(
        receive(
            &mut replica,
            0,
            message(
                1,
                0,
                Body::DoViewChange {
                    retained_view: 0,
                    state: state(vec![], 0),
                },
            ),
        )
        .is_empty()
    );
    assert_replica_unchanged("recovering future DO_VIEW_CHANGE", &before, &replica);
}
