use rand::{Rng, SeedableRng};
use whatoeat::{
    domain::*,
    model::Model,
    store::{Command, Store},
};
const NOW: i64 = 1_800_000_000_000;
fn ready(data: &Data) -> Candidate {
    if let Decision::Ready { candidate, .. } = &data.decision {
        candidate.clone()
    } else {
        panic!("expected candidate")
    }
}
fn known() -> (Data, String) {
    let mut data = Data::default();
    let key = data.add_food("牛肉面").unwrap();
    data.save_meal(None, &key, NOW - 7 * 86_400_000, NOW)
        .unwrap();
    (data, key)
}
#[test]
fn accepts_updates_both_parameters_and_undo_rebuilds() {
    let (mut data, key) = known();
    data.start(NOW, 0.1, false).unwrap();
    let c = ready(&data);
    data.answer(&c.id, true, NOW, 0.1).unwrap();
    let learned = data.food(&key).unwrap().model;
    assert_eq!(learned.samples, 1);
    assert!(learned.a > Model::default().a);
    assert!(learned.b > Model::default().b);
    assert_eq!(data.eaten_count(), 2);
    let meal = if let Decision::Accepted { meal_id, .. } = &data.decision {
        meal_id.clone()
    } else {
        panic!()
    };
    data.remove_meal(&meal).unwrap();
    assert_eq!(data.eaten_count(), 1);
    assert_eq!(data.food(&key).unwrap().model, Model::default());
    data.answer(&c.id, true, NOW, 0.1).unwrap();
    assert_eq!(
        data.eaten_count(),
        1,
        "retry after undo cannot resurrect a meal"
    );
}
#[test]
fn rejection_never_repeats_within_a_round() {
    let mut d = Data::default();
    for name in ["面", "饭", "饺子"] {
        d.add_food(name).unwrap();
    }
    d.start(NOW, 0.1, false).unwrap();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..3 {
        let c = ready(&d);
        assert!(seen.insert(c.food_id.clone()));
        d.answer(&c.id, false, NOW, 0.1).unwrap();
    }
    assert_eq!(d.decision, Decision::Exhausted);
    assert_eq!(d.eaten_count(), 0);
    assert!(d.foods.iter().all(|f| f.model.samples == 0));
    d.start(NOW, 0.1, false).unwrap();
    assert_eq!(d.decision, Decision::Exhausted);
}
#[test]
fn cold_start_needs_a_real_meal_before_training() {
    let mut d = Data::default();
    let key = d.add_food("面").unwrap();
    d.start(NOW, 0.1, false).unwrap();
    let c = ready(&d);
    assert!(c.last_eaten.is_none());
    d.answer(&c.id, true, NOW, 0.1).unwrap();
    assert_eq!(d.food(&key).unwrap().model.samples, 0);
    d.start(NOW + 86_400_000, 0.1, true).unwrap();
    let c = ready(&d);
    d.answer(&c.id, true, NOW + 86_400_000, 0.1).unwrap();
    assert_eq!(d.food(&key).unwrap().model.samples, 1);
}
#[test]
fn editing_meal_revokes_original_acceptance_and_recalculates() {
    let (mut d, key) = known();
    let other = d.add_food("饭").unwrap();
    d.start(NOW, 0.0, false).unwrap();
    let c = ready(&d);
    d.answer(&c.id, true, NOW, 0.1).unwrap();
    let meal = if let Decision::Accepted { meal_id, .. } = &d.decision {
        meal_id.clone()
    } else {
        panic!()
    };
    d.save_meal(Some(&meal), &other, NOW - 1000, NOW).unwrap();
    assert_eq!(d.food(&key).unwrap().model.samples, 0);
    assert_eq!(d.food(&other).unwrap().model.samples, 0);
    assert!(d.feedback[0].revoked);
    assert!(
        d.meals
            .iter()
            .find(|m| m.id == meal)
            .unwrap()
            .feedback_id
            .is_none()
    );
}
#[test]
fn deleting_previous_meal_removes_unlearnable_sample() {
    let (mut d, key) = known();
    let old = d.meals[0].id.clone();
    d.start(NOW, 0.1, false).unwrap();
    let c = ready(&d);
    d.answer(&c.id, false, NOW, 0.1).unwrap();
    assert_eq!(d.food(&key).unwrap().model.samples, 1);
    d.remove_meal(&old).unwrap();
    assert_eq!(d.food(&key).unwrap().model.samples, 0);
}
#[test]
fn disabled_options_and_expired_cards_are_not_accepted() {
    let (mut d, key) = known();
    d.start(NOW, 0.1, false).unwrap();
    let c = ready(&d);
    assert!(d.answer(&c.id, true, NOW + 3 * 3_600_000, 0.1).is_err());
    assert!(d.feedback.is_empty());
    d.set_enabled(&key, false).unwrap();
    d.start(NOW, 0.1, true).unwrap();
    assert_eq!(d.decision, Decision::Exhausted);
}
#[test]
fn backup_roundtrip_rebuilds_and_invalid_backup_is_rejected() {
    let (mut d, key) = known();
    d.start(NOW, 0.1, false).unwrap();
    let c = ready(&d);
    d.answer(&c.id, true, NOW, 0.1).unwrap();
    let restored = Data::from_backup(&d.backup().unwrap(), NOW).unwrap();
    assert_eq!(
        restored.food(&key).unwrap().model,
        d.food(&key).unwrap().model
    );
    assert_eq!(restored.eaten_count(), 2);
    assert_eq!(restored.decision, Decision::Idle);
    let mut invalid = d.clone();
    invalid.foods.push(invalid.foods[0].clone());
    assert!(Data::from_backup(&invalid.backup().unwrap(), NOW).is_err());
    let mut invalid = d.clone();
    invalid.meals[0].food_id = id();
    assert!(Data::from_backup(&invalid.backup().unwrap(), NOW).is_err());
    let mut invalid = d;
    invalid.meals.clear();
    assert!(Data::from_backup(&invalid.backup().unwrap(), NOW).is_err());
}
#[test]
fn sqlite_reopens_the_same_card_and_deduplicates_commands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite3");
    let s = Store::new(&path);
    s.execute("add", Command::AddFood { name: "面".into() }, NOW)
        .unwrap();
    let d = s
        .execute("start", Command::Start { new_round: false }, NOW)
        .unwrap();
    let c = ready(&d);
    assert_eq!(ready(&Store::new(&path).load().unwrap()).id, c.id);
    let cmd = Command::Answer {
        recommendation_id: c.id,
        eat: true,
    };
    s.execute("eat", cmd.clone(), NOW).unwrap();
    s.execute("eat", cmd.clone(), NOW).unwrap();
    s.execute("different-request", cmd, NOW).unwrap();
    let d = s.load().unwrap();
    assert_eq!(d.eaten_count(), 1);
    assert_eq!(d.feedback.len(), 1);
    assert!(
        s.execute("eat", Command::Start { new_round: true }, NOW)
            .is_err()
    );
}
#[test]
fn concurrent_clicks_record_one_meal() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(dir.path().join("db.sqlite3"));
    s.execute("add", Command::AddFood { name: "面".into() }, NOW)
        .unwrap();
    let d = s
        .execute("start", Command::Start { new_round: false }, NOW)
        .unwrap();
    let c = ready(&d);
    let mut threads = vec![];
    for n in 0..4 {
        let s = s.clone();
        let key = c.id.clone();
        threads.push(std::thread::spawn(move || {
            s.execute(
                &format!("eat-{n}"),
                Command::Answer {
                    recommendation_id: key,
                    eat: true,
                },
                NOW,
            )
        }));
    }
    for t in threads {
        t.join().unwrap().unwrap();
    }
    assert_eq!(s.load().unwrap().eaten_count(), 1);
    assert_eq!(s.load().unwrap().feedback.len(), 1);
}
#[test]
fn failed_restore_is_atomic_and_valid_restore_keeps_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(dir.path().join("db.sqlite3"));
    let original = s
        .execute("add", Command::AddFood { name: "面".into() }, NOW)
        .unwrap();
    assert!(
        s.execute(
            "bad",
            Command::Restore {
                backup: "{}".into()
            },
            NOW
        )
        .is_err()
    );
    assert_eq!(s.load().unwrap(), original);
    let mut other = Data::default();
    other.add_food("饭").unwrap();
    s.execute(
        "restore",
        Command::Restore {
            backup: other.backup().unwrap(),
        },
        NOW,
    )
    .unwrap();
    assert_eq!(s.load().unwrap().foods[0].name, "饭");
    let path = std::fs::read_dir(dir.path().join("recovery"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let saved = Data::from_backup(&std::fs::read_to_string(path).unwrap(), NOW).unwrap();
    assert_eq!(saved.foods[0].name, "面");
}
#[test]
fn two_parameter_model_improves_synthetic_held_out_log_loss() {
    let truth = Model {
        a: -5.2,
        b: 3.0,
        samples: 0,
    };
    let prior = Model::default();
    let mut model = prior;
    let mut rng = rand::rngs::StdRng::seed_from_u64(41);
    for _ in 0..10_000 {
        let d = rng.random_range(0.0..30.0);
        model.update(d, rng.random::<f64>() < truth.predict(d));
    }
    let loss = |m: Model| {
        (0..300)
            .map(|n| {
                let d = n as f64 / 10.0;
                let t = truth.predict(d);
                let p = m.predict(d);
                -t * p.ln() - (1.0 - t) * (1.0 - p).ln()
            })
            .sum::<f64>()
            / 300.0
    };
    // Compare reducible error to the generating model rather than an arbitrary
    // absolute loss drop (the prior is already quite close to this truth).
    let initial_excess = loss(prior) - loss(truth);
    assert!(
        loss(model) - loss(truth) < initial_excess * 0.8,
        "prior={} learned={} oracle={}",
        loss(prior),
        loss(model),
        loss(truth)
    );
    assert!((model.b - prior.b).abs() > 0.05);
    assert!(model.predict(1.0) < model.predict(10.0));
}
#[test]
fn replay_is_deterministic_and_does_not_learn_from_own_meal() {
    let (mut d, key) = known();
    d.start(NOW, 0.1, false).unwrap();
    let c = ready(&d);
    d.answer(&c.id, true, NOW, 0.1).unwrap();
    let expected = d.food(&key).unwrap().model;
    let mut direct = Model::default();
    direct.update(7.0, true);
    assert_eq!(expected, direct);
    d.rebuild();
    d.rebuild();
    assert_eq!(d.food(&key).unwrap().model, expected);
}

#[test]
fn rename_keeps_identity_feedback_and_model() {
    let (mut d, key) = known();
    d.start(NOW, 0.0, false).unwrap();
    d.answer(&ready(&d).id, true, NOW, 0.0).unwrap();
    let meals = d.meals.clone();
    let feedback = d.feedback.clone();
    let model = d.food(&key).unwrap().model;
    d.rename_food(&key, "  番茄牛肉面  ").unwrap();
    assert_eq!(d.food(&key).unwrap().name, "番茄牛肉面");
    assert_eq!(d.meals, meals);
    assert_eq!(d.feedback, feedback);
    assert_eq!(d.food(&key).unwrap().model, model);
    d.add_food("饭").unwrap();
    for invalid in ["", "   ", "饭", &"面".repeat(33)] {
        assert!(d.rename_food(&key, invalid).is_err());
    }
    assert_eq!(d.food(&key).unwrap().name, "番茄牛肉面");
}

#[test]
fn deleted_menu_item_preserves_history_and_can_be_recreated_without_reusing_learning() {
    let (mut d, key) = known();
    d.start(NOW, 0.0, false).unwrap();
    let card = ready(&d);
    d.answer(&card.id, true, NOW, 0.0).unwrap();
    let model = d.food(&key).unwrap().model;
    d.start(NOW, 0.0, true).unwrap();
    let stale = ready(&d);
    d.delete_food(&key).unwrap();
    d.delete_food(&key).unwrap();
    assert_eq!(d.menu().count(), 0);
    assert_eq!(d.eaten_count(), 2);
    assert_eq!(d.food(&key).unwrap().model, model);
    assert!(d.set_enabled(&key, true).is_err());
    assert!(d.rename_food(&key, "旧菜单").is_err());
    assert!(d.answer(&stale.id, true, NOW, 0.0).is_err());
    d.start(NOW, 0.0, true).unwrap();
    assert_eq!(d.decision, Decision::Exhausted);
    assert!(d.save_meal(None, &key, NOW, NOW).is_err());
    let old_meal = d.meals[0].id.clone();
    d.save_meal(Some(&old_meal), &key, NOW - 6 * 86_400_000, NOW)
        .unwrap();
    let new_key = d.add_food("牛肉面").unwrap();
    assert_ne!(key, new_key);
    assert_eq!(d.food(&new_key).unwrap().model.samples, 0);
    let restored = Data::from_backup(&d.backup().unwrap(), NOW).unwrap();
    assert_eq!(restored.menu().count(), 1);
    assert_eq!(restored.eaten_count(), 2);
    assert!(restored.food(&key).unwrap().deleted);
}

#[test]
fn old_backups_default_to_non_deleted_menu_items() {
    let (d, key) = known();
    let mut json = serde_json::to_value(d).unwrap();
    for food in json["foods"].as_array_mut().unwrap() {
        food.as_object_mut().unwrap().remove("deleted");
    }
    let restored = Data::from_backup(&json.to_string(), NOW).unwrap();
    assert_eq!(restored.menu().count(), 1);
    assert!(!restored.food(&key).unwrap().deleted);
}

#[test]
fn menu_changes_persist_and_deleted_options_do_not_return_on_restart() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("db.sqlite3"));
    let d = store
        .execute("add", Command::AddFood { name: "面".into() }, NOW)
        .unwrap();
    let key = d.foods[0].id.clone();
    store
        .execute(
            "rename",
            Command::RenameFood {
                food_id: key.clone(),
                name: "拌面".into(),
            },
            NOW,
        )
        .unwrap();
    assert_eq!(store.load().unwrap().food(&key).unwrap().name, "拌面");
    let delete = Command::DeleteFood {
        food_id: key.clone(),
    };
    store.execute("delete", delete.clone(), NOW).unwrap();
    store.execute("retry", delete, NOW).unwrap();
    assert_eq!(store.load().unwrap().menu().count(), 0);
    assert!(
        store
            .execute(
                "enable",
                Command::SetEnabled {
                    food_id: key,
                    enabled: true
                },
                NOW
            )
            .is_err()
    );
    assert_eq!(store.load().unwrap().menu().count(), 0);
}
