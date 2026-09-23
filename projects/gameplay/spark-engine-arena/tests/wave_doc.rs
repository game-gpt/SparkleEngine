use spark_engine_arena::{WaveDocument, timeline_from_json};

#[test]
fn parse_wave_json_expands_count() {
    let json = r#"{"version":1,"events":[{"at_secs":1.0,"tag":2,"count":3}]}"#;
    let doc = WaveDocument::from_json(json).expect("parse");
    assert_eq!(doc.version, 1);
    let tl = doc.into_timeline();
    let tags: Vec<u32> = tl.drain_due(10.0).collect();
    assert_eq!(tags, vec![2, 2, 2]);
}

#[test]
fn timeline_from_json_helper() {
    let json = r#"{"version":1,"events":[{"at_secs":0.5,"tag":1}]}"#;
    let tl = timeline_from_json(json).expect("parse");
    assert_eq!(tl.drain_due(1.0).collect::<Vec<_>>(), vec![1]);
}
