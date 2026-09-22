use std::collections::{BTreeMap, BTreeSet};
use whatoeat::{
    i18n::Language,
    store::{Command, Store},
};

#[test]
fn supported_language_follows_order_and_understands_native_locale_tags() {
    for tag in ["zh-CN", "zh-HK", "zh_TW.UTF-8", "ZH_cn@variant"] {
        assert_eq!(Language::from_preferences([tag]), Language::Chinese);
    }
    assert_eq!(
        Language::from_preferences(["fr-FR", "zh-CN", "en"]),
        Language::Chinese
    );
    assert_eq!(
        Language::from_preferences(["en_US.UTF-8", "zh-CN"]),
        Language::English
    );
    assert_eq!(
        Language::from_preferences(["C.UTF-8", "ja"]),
        Language::English
    );
    assert_eq!(Language::from_preferences([]), Language::English);
}

#[test]
fn notices_translate_at_display_time_without_losing_diagnostic_details() {
    let message = "本地数据库操作失败：database is locked";
    assert_eq!(Language::Chinese.message(message), message);
    assert_eq!(
        Language::English.message(message),
        "Local database error: database is locked"
    );
    assert_eq!(
        Language::English.message("主题未能保存：文件操作失败：Permission denied"),
        "Could not save the theme: File operation failed: Permission denied"
    );
    assert_eq!(
        Language::English.message("这次记录和学习反馈已撤销。"),
        "This meal and its learning feedback have been undone."
    );
    assert_eq!(Language::English.text("私房红烧肉"), "私房红烧肉");
}

#[test]
fn translations_preserve_all_named_placeholders() {
    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split('{')
            .skip(1)
            .filter_map(|s| s.split_once('}').map(|(key, _)| key))
            .collect()
    }
    let catalog: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../assets/locales/en.json")).unwrap();
    for (source, translation) in catalog {
        assert!(
            !translation.trim().is_empty(),
            "empty translation: {source}"
        );
        assert_eq!(
            placeholders(&source),
            placeholders(&translation),
            "placeholder mismatch: {source}"
        );
    }
    assert_eq!(
        Language::English.format("参考周期 {cycle}", &[("cycle", "7 days".into())]),
        "Interval: 7 days"
    );
}

#[test]
fn english_starters_are_idempotent_and_never_rename_existing_foods_or_invent_history() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("whatoeat.sqlite3"));
    let now = 1_800_000_000_000;
    store
        .execute(
            "custom",
            Command::AddFood {
                name: "饺子".into(),
            },
            now,
        )
        .unwrap();
    store
        .execute("seed", Command::AddEnglishStarters, now)
        .unwrap();
    let data = store
        .execute("seed-again", Command::AddEnglishStarters, now)
        .unwrap();
    assert_eq!(data.menu().count(), 7);
    assert!(data.menu().any(|f| f.name == "饺子"));
    assert!(data.menu().any(|f| f.name == "Dumplings"));
    assert!(data.meals.is_empty());
    assert!(data.feedback.is_empty());
    // The old receipt representation remains readable after this update.
    assert!(matches!(
        serde_json::from_str::<Command>("\"AddStarters\"").unwrap(),
        Command::AddStarters
    ));
}
