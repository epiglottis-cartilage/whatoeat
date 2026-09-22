use rusqlite::{Connection, params};
use serde_json::{Value, json};
use whatoeat::{
    domain::*,
    store::{Command, Store},
};
const NOW: i64 = 1_800_000_000_000;

fn learned() -> (Data, String, String) {
    let mut data = Data::default();
    let key = data.add_food("牛肉面").unwrap();
    data.save_meal(None, &key, NOW - 7 * 86_400_000, NOW)
        .unwrap();
    data.start(NOW - 1000, 0.0, true).unwrap();
    let Decision::Ready { candidate, .. } = data.decision.clone() else {
        panic!()
    };
    data.answer(&candidate.id, true, NOW, 0.0).unwrap();
    (data, key, candidate.id)
}
fn compact(menu: Value, records: Value, rejected: Value) -> String {
    json!({"version":2,"menu":menu,"records":records,"rejected":rejected}).to_string()
}
fn legacy(data: &Data) -> String {
    let mut value = serde_json::to_value(data).unwrap();
    value["profile_id"] = id().into();
    if let Some(ready) = value.pointer_mut("/decision/Ready") {
        ready["session_id"] = id().into();
    }
    for index in 0..value["feedback"].as_array().unwrap().len() {
        let recommendation = value["feedback"][index]["id"].clone();
        let old_id: Value = id().into();
        for meal in value["meals"].as_array_mut().unwrap() {
            if meal["feedback_id"] == recommendation {
                meal["feedback_id"] = old_id.clone();
            }
        }
        value["feedback"][index]["recommendation_id"] = recommendation;
        value["feedback"][index]["id"] = old_id;
    }
    value.to_string()
}

#[test]
fn portable_roundtrip_has_no_ids_and_keeps_learning_and_undo() {
    let (source, key, _) = learned();
    let text = source.export_text().unwrap();
    assert!(!text.contains("_id"));
    assert!(!text.contains("\"id\""));
    assert!(!text.contains("model"));
    let mut target = Data::default();
    target.merge_text(&text, NOW).unwrap();
    assert_eq!(target.eaten_count(), 2);
    assert_ne!(target.foods[0].id, key);
    assert_eq!(target.foods[0].model, source.food(&key).unwrap().model);
    let before = target.clone();
    target.merge_text(&text, NOW).unwrap();
    assert_eq!(target, before);
    let accepted = target
        .meals
        .iter()
        .find(|m| m.feedback_id.is_some())
        .unwrap()
        .id
        .clone();
    target.remove_meal(&accepted).unwrap();
    assert_eq!(target.foods[0].model.samples, 0);
    target.merge_text(&text, NOW).unwrap();
    assert_eq!(
        target.eaten_count(),
        2,
        "explicit import overrides the matching local deletion"
    );
    assert_eq!(target.foods[0].model.samples, 1);
}

#[test]
fn merge_matches_names_and_minutes_and_overwrites_menu_state() {
    let mut data = Data::default();
    let key = data.add_food("Noodles").unwrap();
    data.set_enabled(&key, false).unwrap();
    data.save_meal(None, &key, NOW - 1000, NOW).unwrap();
    let text = compact(
        json!([{"name":" noodles ","enabled":true}, {"name":"饺子","enabled":true}]),
        json!([{"name":"NOODLES","eaten_at":NOW-1000}, {"name":" noodles ","eaten_at":NOW-1000}, {"name":"noodles","eaten_at":NOW-999}, {"name":"饺子","eaten_at":NOW}]),
        json!([]),
    );
    data.merge_text(&text, NOW).unwrap();
    assert_eq!(data.foods.len(), 2);
    assert_eq!(data.eaten_count(), 2);
    assert!(data.food(&key).unwrap().enabled);
    assert_eq!(data.food(&key).unwrap().name, "noodles");
    let before = data.clone();
    data.merge_text(&text, NOW).unwrap();
    assert_eq!(data, before);
}

#[test]
fn imports_overwrite_matching_deletions() {
    let (mut data, key, _) = learned();
    let original = data.export_text().unwrap();
    let meal = data.meals[0].id.clone();
    data.remove_meal(&meal).unwrap();
    data.delete_food(&key).unwrap();
    data.merge_text(&original, NOW).unwrap();
    assert_eq!(data.menu().count(), 1);
    assert_eq!(data.eaten_count(), 2);
    let mut empty = Data::default();
    empty.merge_text(&data.export_text().unwrap(), NOW).unwrap();
    assert_eq!(empty.menu().count(), 1);
    assert_eq!(empty.eaten_count(), 2);
}

#[test]
fn incremental_import_preserves_records_at_other_minutes() {
    let (mut data, key, _) = learned();
    let original = data.export_text().unwrap();
    let meal = data.meals[1].id.clone();
    data.save_meal(Some(&meal), &key, NOW - MINUTE, NOW)
        .unwrap();
    data.merge_text(&original, NOW).unwrap();
    assert_eq!(data.eaten_count(), 3);
    assert_eq!(data.food(&key).unwrap().model.samples, 1);
    assert!(
        data.meals
            .iter()
            .any(|m| !m.deleted && m.eaten_at == NOW - MINUTE)
    );
    assert!(data.meals.iter().any(|m| !m.deleted && m.eaten_at == NOW));
}

#[test]
fn merge_replays_rejections_in_time_order_and_deduplicates_feedback() {
    let menu = json!([{"name":"面","enabled":true}]);
    let first = compact(
        menu.clone(),
        json!([{"name":"面","eaten_at":NOW-7*86_400_000}]),
        json!([{"name":"面","shown_at":NOW-100,"answered_at":NOW-50}]),
    );
    let second = compact(
        menu,
        json!([]),
        json!([{"name":"面","shown_at":NOW-2000,"answered_at":NOW-1500}]),
    );
    let mut left = Data::default();
    left.merge_text(&first, NOW).unwrap();
    left.merge_text(&second, NOW).unwrap();
    left.merge_text(&first, NOW).unwrap();
    let mut right = Data::default();
    right.merge_text(&second, NOW).unwrap();
    right.merge_text(&first, NOW).unwrap();
    assert_eq!(left.feedback.len(), 2);
    assert_eq!(left.foods[0].model.samples, 2);
    assert_eq!(left.foods[0].model, right.foods[0].model);
}

#[test]
fn acceptance_enriches_a_duplicate_manual_meal_without_counting_it_twice() {
    let (source, _, _) = learned();
    let mut target = Data::default();
    let key = target.add_food("牛肉面").unwrap();
    for meal in &source.meals {
        target.save_meal(None, &key, meal.eaten_at, NOW).unwrap();
    }
    target
        .merge_text(&source.export_text().unwrap(), NOW)
        .unwrap();
    assert_eq!(target.eaten_count(), 2);
    assert_eq!(target.feedback.len(), 1);
    assert_eq!(target.foods[0].model, source.foods[0].model);
}

#[test]
fn invalid_merge_is_atomic_in_memory_and_in_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("db.sqlite3"));
    let mut data = store
        .execute("add", Command::AddFood { name: "饭".into() }, NOW)
        .unwrap();
    let original = data.clone();
    let text = compact(
        json!([{"name":"新选项","enabled":true}]),
        json!([{"name":"新选项","eaten_at":NOW+1}]),
        json!([]),
    );
    assert!(data.merge_text(&text, NOW).is_err());
    assert_eq!(data, original);
    assert!(store.execute("bad", Command::Merge { text }, NOW).is_err());
    assert_eq!(store.load().unwrap(), original);
    let valid = compact(
        json!([{"name":"新选项","enabled":true}]),
        json!([]),
        json!([]),
    );
    store
        .execute(
            "good",
            Command::Merge {
                text: valid.clone(),
            },
            NOW,
        )
        .unwrap();
    store
        .execute("again", Command::Merge { text: valid }, NOW)
        .unwrap();
    assert_eq!(store.load().unwrap().menu().count(), 2);
    assert!(
        std::fs::read_dir(dir.path().join("recovery"))
            .unwrap()
            .count()
            >= 1
    );
}

#[test]
fn legacy_database_migrates_ids_and_preserves_card_retry_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let (mut source, key, answered_card) = learned();
    source.start(NOW, 0.0, true).unwrap();
    let raw = legacy(&source);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE app_state(id INTEGER PRIMARY KEY, payload TEXT); PRAGMA user_version=1;",
    )
    .unwrap();
    conn.execute("INSERT INTO app_state VALUES(1,?1)", params![raw])
        .unwrap();
    drop(conn);
    let store = Store::new(&path);
    let mut migrated = store.load().unwrap();
    assert_eq!(migrated, source);
    migrated.answer(&answered_card, true, NOW, 0.0).unwrap();
    assert_eq!(migrated.eaten_count(), 2);
    let meal = migrated.meals[1].id.clone();
    migrated.remove_meal(&meal).unwrap();
    assert_eq!(migrated.food(&key).unwrap().model.samples, 0);
    let conn = Connection::open(&path).unwrap();
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 3);
    let stored: String = conn
        .query_row("SELECT payload FROM app_state", [], |r| r.get(0))
        .unwrap();
    for field in ["profile_id", "session_id", "recommendation_id"] {
        assert!(!stored.contains(field));
    }
    let files: Vec<_> = std::fs::read_dir(dir.path().join("recovery"))
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1);
    let backup = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(backup, raw);
    assert_eq!(store.load().unwrap(), source);
    assert_eq!(
        std::fs::read_dir(dir.path().join("recovery"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn legacy_backups_merge_by_name_even_with_unrelated_ids() {
    let (source, _, _) = learned();
    let mut target = Data::default();
    target.add_food("牛肉面").unwrap();
    target.add_food("饺子").unwrap();
    target.merge_text(&legacy(&source), NOW).unwrap();
    target
        .merge_text(&source.export_text().unwrap(), NOW)
        .unwrap();
    assert_eq!(target.foods.len(), 2);
    assert_eq!(target.eaten_count(), 2);
    assert_eq!(target.feedback.len(), 1);
}

#[test]
fn invalid_versions_unknown_names_and_broken_feedback_are_rejected() {
    let (mut data, _, _) = learned();
    let original = data.clone();
    let mut bad: Value = serde_json::from_str(&data.export_text().unwrap()).unwrap();
    bad["version"] = 99.into();
    assert!(data.merge_text(&bad.to_string(), NOW).is_err());
    bad["version"] = 2.into();
    bad["records"][0]["name"] = "没有这个菜单".into();
    assert!(data.merge_text(&bad.to_string(), NOW).is_err());
    bad["records"][0]["name"] = "牛肉面".into();
    bad["records"][0]["shown_at"] = (NOW + 1).into();
    assert!(data.merge_text(&bad.to_string(), NOW).is_err());
    assert_eq!(data, original);
}

#[test]
fn failed_legacy_migration_preserves_payload_and_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let (data, _, _) = learned();
    let mut malformed: Value = serde_json::from_str(&legacy(&data)).unwrap();
    malformed["meals"][0]["food_id"] = id().into();
    let raw = malformed.to_string();
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE app_state(id INTEGER PRIMARY KEY, payload TEXT); PRAGMA user_version=1;",
    )
    .unwrap();
    conn.execute("INSERT INTO app_state VALUES(1,?1)", params![raw])
        .unwrap();
    drop(conn);
    assert!(Store::new(&path).load().is_err());
    let conn = Connection::open(path).unwrap();
    let stored: String = conn
        .query_row("SELECT payload FROM app_state", [], |r| r.get(0))
        .unwrap();
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(stored, raw);
    assert_eq!(version, 1);
}

#[test]
fn minute_collision_replaces_acceptance_and_repeated_import_is_stable() {
    let (mut data, _, _) = learned();
    let incoming = compact(
        json!([{"name":"牛肉面","enabled":false}]),
        json!([{"name":"牛肉面","eaten_at":NOW+10_000}]),
        json!([]),
    );
    data.merge_text(&incoming, NOW + 30_000).unwrap();
    assert_eq!(data.eaten_count(), 2);
    assert!(!data.foods[0].enabled);
    assert_eq!(data.foods[0].model.samples, 0);
    assert!(
        data.meals
            .iter()
            .find(|m| m.eaten_at == NOW && !m.deleted)
            .unwrap()
            .feedback_id
            .is_none()
    );
    let before = data.clone();
    data.merge_text(&incoming, NOW + 30_000).unwrap();
    assert_eq!(data, before);
}

#[test]
fn minute_precision_applies_to_acceptance_manual_entry_and_roundtrip() {
    let mut data = Data::default();
    let food = data.add_food("面").unwrap();
    data.save_meal(None, &food, NOW - 7 * 86_400_000 + 12_345, NOW)
        .unwrap();
    data.start(NOW + 23_000, 0.0, true).unwrap();
    let Decision::Ready { candidate, .. } = data.decision.clone() else {
        panic!()
    };
    data.answer(&candidate.id, true, NOW + 34_567, 0.0).unwrap();
    assert!(data.meals.iter().all(|m| m.eaten_at % MINUTE == 0));
    assert_eq!(data.meals[1].eaten_at, NOW);
    let exported = data.export_text().unwrap();
    let mut restored = Data::default();
    restored.merge_text(&exported, NOW + 34_567).unwrap();
    assert_eq!(restored.foods[0].model, data.foods[0].model);
    assert_eq!(restored.feedback[0].shown_at, NOW + 23_000);
    assert_eq!(restored.feedback[0].answered_at, NOW + 34_567);
    data.save_meal(None, &food, NOW + 55_000, NOW + 55_000)
        .unwrap();
    assert_eq!(data.eaten_count(), 2, "same name/minute is one meal");
    assert_eq!(
        data.foods[0].model.samples, 0,
        "replacement revokes the old acceptance"
    );
}

#[test]
fn rounding_meal_time_does_not_train_an_earlier_rejection_on_a_future_meal() {
    let mut data = Data::default();
    data.add_food("面").unwrap();
    data.start(NOW + 10_000, 0.0, true).unwrap();
    let Decision::Ready { candidate, .. } = data.decision.clone() else {
        panic!()
    };
    data.answer(&candidate.id, false, NOW + 15_000, 0.0)
        .unwrap();
    data.start(NOW + 20_000, 0.0, true).unwrap();
    let Decision::Ready { candidate, .. } = data.decision.clone() else {
        panic!()
    };
    data.answer(&candidate.id, true, NOW + 30_000, 0.0).unwrap();
    assert_eq!(data.eaten_count(), 1);
    assert_eq!(data.meals[0].eaten_at, NOW);
    assert_eq!(data.foods[0].model.samples, 0);
}

#[test]
fn clear_removes_all_facts_and_old_receipts_and_can_start_fresh() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite3");
    let store = Store::new(&path);
    let (data, _, _) = learned();
    store
        .execute(
            "import",
            Command::Merge {
                text: data.export_text().unwrap(),
            },
            NOW,
        )
        .unwrap();
    let cleared = store.execute("clear", Command::Clear, NOW).unwrap();
    assert!(cleared.foods.is_empty() && cleared.meals.is_empty() && cleared.feedback.is_empty());
    assert_eq!(cleared.decision, Decision::Idle);
    assert_eq!(store.load().unwrap(), cleared);
    let conn = Connection::open(path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM command_receipts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "only the empty clear-command receipt remains");
    let fresh = store
        .execute(
            "new",
            Command::AddFood {
                name: "饺子".into(),
            },
            NOW,
        )
        .unwrap();
    let retry = store.execute("clear", Command::Clear, NOW).unwrap();
    assert_eq!(
        retry, fresh,
        "a retry of the original clear cannot erase later data"
    );
}

#[test]
fn v2_migration_rounds_legacy_meals_and_resolves_minute_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.sqlite3");
    let mut data = Data::default();
    let food = data.add_food("面").unwrap();
    data.save_meal(None, &food, NOW - 2 * MINUTE, NOW).unwrap();
    data.save_meal(None, &food, NOW - MINUTE, NOW).unwrap();
    data.meals[0].eaten_at = NOW - 15_000;
    data.meals[1].eaten_at = NOW - 5_000;
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE app_state(id INTEGER PRIMARY KEY, payload TEXT); PRAGMA user_version=2;",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO app_state VALUES(1,?1)",
        [data.backup().unwrap()],
    )
    .unwrap();
    drop(conn);
    let migrated = Store::new(&path).load().unwrap();
    assert_eq!(migrated.eaten_count(), 1);
    assert!(migrated.meals.iter().all(|m| m.eaten_at == NOW - MINUTE));
    assert!(migrated.meals[0].deleted);
    assert!(!migrated.meals[1].deleted);
    let conn = Connection::open(path).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
}
