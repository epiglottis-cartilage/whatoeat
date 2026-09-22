use rusqlite::Connection;
use whatoeat::{
    domain::Data,
    store::{Command, Store},
    theme::Theme,
};

const NOW: i64 = 1_800_000_000_000;

#[test]
fn theme_persists_after_reopening_without_changing_food_data() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("whatoeat.sqlite3");
    let store = Store::new(&path);
    assert_eq!(store.load_theme().unwrap(), Theme::Poster);
    let original = store
        .execute(
            "add",
            Command::AddFood {
                name: "饺子".into(),
            },
            NOW,
        )
        .unwrap();
    for theme in Theme::ALL {
        assert_eq!(Theme::from_key(theme.key()), Some(theme));
        store.save_theme(theme).unwrap();
        let reopened = Store::new(&path);
        assert_eq!(reopened.load_theme().unwrap(), theme);
        assert_eq!(reopened.load().unwrap(), original);
    }
}

#[test]
fn unknown_theme_falls_back_to_poster() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("whatoeat.sqlite3");
    let store = Store::new(&path);
    store.save_theme(Theme::Rhodes).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE app_settings SET value='future-theme' WHERE key='theme'",
            [],
        )
        .unwrap();
    assert_eq!(store.load_theme().unwrap(), Theme::Poster);
    assert_eq!(Theme::from_key("future-theme"), None);
}

#[test]
fn clear_merge_and_restore_keep_the_devices_theme() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("whatoeat.sqlite3"));
    store.save_theme(Theme::Rhine).unwrap();
    let original = store
        .execute(
            "add",
            Command::AddFood {
                name: "饺子".into(),
            },
            NOW,
        )
        .unwrap();
    let cleared = store.execute("clear", Command::Clear, NOW).unwrap();
    assert_eq!(cleared.menu().count(), 0);
    assert_eq!(store.load_theme().unwrap(), Theme::Rhine);
    let merged = store
        .execute(
            "merge",
            Command::Merge {
                text: original.export_text().unwrap(),
            },
            NOW,
        )
        .unwrap();
    assert_eq!(merged.menu().count(), 1);
    assert_eq!(store.load_theme().unwrap(), Theme::Rhine);
    let restored = store
        .execute(
            "restore",
            Command::Restore {
                backup: Data::default().backup().unwrap(),
            },
            NOW,
        )
        .unwrap();
    assert_eq!(restored.menu().count(), 0);
    assert_eq!(store.load_theme().unwrap(), Theme::Rhine);
}

#[test]
fn version_three_database_adds_preferences_without_a_version_bump() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("whatoeat.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE app_state (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
             CREATE TABLE command_receipts (request_id TEXT PRIMARY KEY, command TEXT NOT NULL);
             PRAGMA user_version=3;",
        )
        .unwrap();
    let original = Data::default().backup().unwrap();
    connection
        .execute("INSERT INTO app_state VALUES(1,?1)", [&original])
        .unwrap();
    let store = Store::new(&path);
    assert_eq!(store.load_theme().unwrap(), Theme::Poster);
    store.save_theme(Theme::Penguin).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 3);
    let payload: String = connection
        .query_row("SELECT payload FROM app_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(payload, original);
    assert_eq!(store.load_theme().unwrap(), Theme::Penguin);
}
