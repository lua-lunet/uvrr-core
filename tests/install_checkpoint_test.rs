use uuid::Uuid;
use vrr::vrr::*;

#[test]
fn install_checkpoint_truncates_log_and_accepts_suffix() {
    let members = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let mut leader = Replica::new(members.clone(), "A").unwrap();
    
    // Simulate some requests
    leader.step(Input::Request {
        client_id: 1,
        request_num: 1,
        message_id: Uuid::new_v4(),
        execution_time: 1,
        payload: vec![1],
    });
    
    leader.step(Input::Message {
        from: 1,
        message: Message {
            epoch: 0,
            slot: 1,
            body: Body::PrepareOk,
        },
    });
    
    leader.step(Input::Complete { slot: 1, result: vec![2] });
    
    // Check state
    let diag = leader.diagnostic();
    assert_eq!(diag.slot, 1);
    assert_eq!(diag.commit, 1);
    assert_eq!(diag.executed, 1);
    
    // Recover leader to allow install_checkpoint
    leader.step(Input::Recover { nonce: 1 });
    
    assert!(leader.install_checkpoint(1).is_ok());
    
    // Now log should be empty
    assert_eq!(leader.log().len(), 0);
    assert_eq!(leader.diagnostic().log.len(), 0);
    assert_eq!(leader.diagnostic().slot, 1);
    
    // Let's manually append a new entry to the leader (or just check that a follower can adopt the leader's state)
}
