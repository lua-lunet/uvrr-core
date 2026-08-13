mod support;

use support::{
    Relation, ReplicaSnapshot, VIEW, assert_complete_cases, assert_replica_unchanged, entry,
    message, node, receive, state,
};
use vrr::vrr::{Body, Input, LogState, NodeId};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
enum ViewChangeMessage {
    StartViewChange,
    DoViewChange,
    StartView,
}

impl ViewChangeMessage {
    const ALL: [Self; 3] = [Self::StartViewChange, Self::DoViewChange, Self::StartView];

    fn carries_state(self) -> bool {
        !matches!(self, Self::StartViewChange)
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
enum SenderEligibility {
    Eligible,
    Ineligible,
}

impl SenderEligibility {
    const ALL: [Self; 2] = [Self::Eligible, Self::Ineligible];
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
enum StateValidity {
    Valid,
    Invalid,
}

impl StateValidity {
    const ALL: [Self; 2] = [Self::Valid, Self::Invalid];
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
struct Case {
    message: ViewChangeMessage,
    view: Relation,
    sender: SenderEligibility,
    state: Option<StateValidity>,
}

fn sender(case: Case, view: u32) -> NodeId {
    let leader = view % 4;
    match (case.message, case.sender) {
        (ViewChangeMessage::StartViewChange, SenderEligibility::Eligible) => 0,
        (ViewChangeMessage::StartViewChange, SenderEligibility::Ineligible) => 4,
        (_, SenderEligibility::Eligible) => leader,
        (_, SenderEligibility::Ineligible) => (0..4)
            .find(|node| *node != leader && *node != 3)
            .expect("four-node replica has a non-leader peer"),
    }
}

fn transferred_state(validity: StateValidity) -> LogState {
    match validity {
        StateValidity::Valid => state(vec![entry(1, 1, 1)], 0),
        StateValidity::Invalid => LogState {
            slot: 2,
            commit: 0,
            log: vec![entry(1, 1, 1)],
        },
    }
}

fn body(case: Case) -> Body {
    match case.message {
        ViewChangeMessage::StartViewChange => Body::StartViewChange,
        ViewChangeMessage::DoViewChange => Body::DoViewChange {
            retained_view: 0,
            state: transferred_state(case.state.expect("DO_VIEW_CHANGE carries state")),
        },
        ViewChangeMessage::StartView => Body::StartView {
            state: transferred_state(case.state.expect("START_VIEW carries state")),
        },
    }
}

#[test]
fn recovering_replica_isolates_the_complete_view_change_matrix() {
    let cases: Vec<_> = ViewChangeMessage::ALL
        .into_iter()
        .flat_map(|message| {
            Relation::ALL.into_iter().flat_map(move |view| {
                SenderEligibility::ALL.into_iter().flat_map(move |sender| {
                    if message.carries_state() {
                        StateValidity::ALL
                            .into_iter()
                            .map(move |state| Case {
                                message,
                                view,
                                sender,
                                state: Some(state),
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![Case {
                            message,
                            view,
                            sender,
                            state: None,
                        }]
                    }
                })
            })
        })
        .collect();
    assert_complete_cases(
        "recovering view-change isolation",
        30,
        cases.iter().copied(),
    );

    for case in cases {
        let mut replica = node(4, 3);
        assert!(
            receive(
                &mut replica,
                1,
                message(
                    VIEW,
                    0,
                    Body::StartView {
                        state: state(vec![], 0)
                    }
                ),
            )
            .is_empty()
        );
        assert_eq!(replica.view(), VIEW);
        assert_eq!(replica.step(Input::Recover { nonce: 7 }).len(), 1);
        let before = ReplicaSnapshot::capture(&replica);

        let view = case.view.view();
        let body = body(case);
        let slot = match &body {
            Body::DoViewChange { state, .. } | Body::StartView { state } => state.slot,
            Body::StartViewChange => 0,
            _ => unreachable!("matrix contains only view-change messages"),
        };
        let outputs = receive(&mut replica, sender(case, view), message(view, slot, body));

        assert!(
            outputs.is_empty(),
            "recovering case {case:?} produced output"
        );
        assert_replica_unchanged(case, &before, &replica);
    }
}
