use sharo_core::coordination::{CoordinationConflictRecord, CoordinationIntentRecord};

#[test]
fn coordination_record_schema_roundtrip() {
    let intent = CoordinationIntentRecord {
        intent_id: "intent-000001".to_string(),
        task_id: "task-000001".to_string(),
        scope: "notes".to_string(),
        goal: "draft scope:notes release".to_string(),
    };

    let json = serde_json::to_string(&intent).expect("serialize intent");
    let parsed: CoordinationIntentRecord = serde_json::from_str(&json).expect("parse intent");
    assert_eq!(parsed, intent);
}

#[test]
fn coordination_record_ids_are_stable() {
    let first = CoordinationConflictRecord {
        conflict_id: "conflict-000010".to_string(),
        task_id: "task-000010".to_string(),
        related_task_id: "task-000009".to_string(),
        scope: "notes".to_string(),
    };
    let second = first.clone();
    assert_eq!(first.conflict_id, second.conflict_id);
}
