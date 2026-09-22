//! Portable, name-based exchange. Internal identity never crosses this boundary.
use crate::domain::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::Invalid(message.into())
}
fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transfer {
    version: u32,
    menu: Vec<MenuEntry>,
    records: Vec<Record>,
    rejected: Vec<Rejection>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MenuEntry {
    name: String,
    enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    deleted: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    name: String,
    eaten_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shown_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    answered_at: Option<Timestamp>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rejection {
    name: String,
    shown_at: Timestamp,
    answered_at: Timestamp,
}

impl Transfer {
    fn from_data(data: &Data) -> Self {
        let mut menu: BTreeMap<String, MenuEntry> = BTreeMap::new();
        for food in &data.foods {
            let entry = MenuEntry {
                name: food.name.clone(),
                enabled: food.enabled,
                deleted: food.deleted,
            };
            let key = name_key(&food.name);
            // A live option takes precedence over earlier deleted namesakes.
            if menu.get(&key).is_none_or(|previous| previous.deleted) {
                menu.insert(key, entry);
            }
        }
        let names: HashMap<_, _> = data
            .foods
            .iter()
            .map(|f| (f.id.as_str(), f.name.as_str()))
            .collect();
        let feedback: HashMap<_, _> = data.feedback.iter().map(|f| (f.id.as_str(), f)).collect();
        let records = data
            .meals
            .iter()
            .filter(|m| !m.deleted)
            .map(|m| Record {
                name: names[m.food_id.as_str()].into(),
                eaten_at: minute(m.eaten_at),
                answered_at: m
                    .feedback_id
                    .as_deref()
                    .and_then(|key| feedback.get(key))
                    .filter(|f| !f.revoked && f.eat)
                    .map(|f| f.answered_at),
                shown_at: m
                    .feedback_id
                    .as_deref()
                    .and_then(|key| feedback.get(key))
                    .filter(|f| !f.revoked && f.eat)
                    .map(|f| f.shown_at),
            })
            .collect();
        let rejected = data
            .feedback
            .iter()
            .filter(|f| !f.eat && !f.revoked)
            .map(|f| Rejection {
                name: names[f.food_id.as_str()].into(),
                shown_at: f.shown_at,
                answered_at: f.answered_at,
            })
            .collect();
        Self {
            version: 3,
            menu: menu.into_values().collect(),
            records,
            rejected,
        }
    }

    fn parse(text: &str, now: Timestamp) -> Result<Self> {
        if text.len() > MAX_BACKUP_BYTES {
            return Err(invalid("导入内容不能超过 16 MB。"));
        }
        let value: serde_json::Value = serde_json::from_str(text)?;
        let mut transfer = if value.get("format_version").is_some() {
            Self::from_data(&Data::from_backup(text, now)?)
        } else {
            serde_json::from_value::<Self>(value)?
        };
        transfer.validate(now)?;
        for record in &mut transfer.records {
            if let Some(shown_at) = record.shown_at
                && record.answered_at.is_none()
            {
                record.answered_at = Some(record.eaten_at.max(shown_at));
            }
            record.eaten_at = minute(record.eaten_at);
        }
        Ok(transfer)
    }

    fn validate(&self, now: Timestamp) -> Result<()> {
        if ![2, 3].contains(&self.version) {
            return Err(invalid("不支持这个导入版本。"));
        }
        if self.menu.len() > 500 || self.records.len() > 50_000 || self.rejected.len() > 50_000 {
            return Err(invalid("导入记录数量超出当前版本支持范围。"));
        }
        let mut names = HashSet::new();
        for food in &self.menu {
            if food.name.trim().is_empty()
                || food.name.trim().chars().count() > 32
                || (food.deleted && food.enabled)
            {
                return Err(invalid("导入内容包含无效的菜单名称或状态。"));
            }
            names.insert(name_key(&food.name));
        }
        for record in &self.records {
            if !names.contains(&name_key(&record.name))
                || record.eaten_at < 0
                || record.eaten_at > now
                || record.shown_at.is_some_and(|shown| {
                    shown < 0 || shown > now || minute(shown) > minute(record.eaten_at)
                })
                || record.answered_at.is_some_and(|answered| {
                    record.shown_at.is_none_or(|shown| answered < shown)
                        || answered > now
                        || minute(answered) != minute(record.eaten_at)
                })
            {
                return Err(invalid("导入内容包含无效的食用记录。"));
            }
        }
        for feedback in &self.rejected {
            if !names.contains(&name_key(&feedback.name))
                || feedback.shown_at < 0
                || feedback.answered_at < feedback.shown_at
                || feedback.answered_at > now
            {
                return Err(invalid("导入内容包含无效的反馈。"));
            }
        }
        Ok(())
    }
}

impl Data {
    pub fn export_text(&self) -> Result<String> {
        // Also verifies references before constructing name-based records.
        self.validate(i64::MAX)?;
        let mut normalized = self.clone();
        normalized.normalize_meal_minutes();
        Ok(serde_json::to_string_pretty(&Transfer::from_data(
            &normalized,
        ))?)
    }

    /// Validate and merge into a copy; errors never partially mutate the caller.
    pub fn merge_text(&mut self, text: &str, now: Timestamp) -> Result<()> {
        let incoming = Transfer::parse(text, now)?;
        let mut merged = self.clone();
        merged.normalize_meal_minutes();
        merged.merge_transfer(incoming)?;
        merged.validate(now)?;
        merged.decision = Decision::Idle;
        merged.rebuild();
        *self = merged;
        Ok(())
    }

    fn merge_transfer(&mut self, incoming: Transfer) -> Result<()> {
        let mut foods: HashMap<String, String> = HashMap::new();
        for food in &self.foods {
            let key = name_key(&food.name);
            if !foods.contains_key(&key) || !food.deleted {
                foods.insert(key, food.id.clone());
            }
        }
        for food in incoming.menu {
            let key = name_key(&food.name);
            let food_id = if let Some(food_id) = foods.get(&key) {
                food_id.clone()
            } else {
                let food_id = self.add_food(&food.name)?;
                foods.insert(key, food_id.clone());
                food_id
            };
            let target = self.foods.iter_mut().find(|f| f.id == food_id).unwrap();
            target.name = food.name.trim().into();
            target.enabled = food.enabled;
            target.deleted = food.deleted;
        }
        let names: HashMap<_, _> = self
            .foods
            .iter()
            .map(|f| (f.id.clone(), name_key(&f.name)))
            .collect();
        let mut records = HashMap::new();
        for (index, meal) in self.meals.iter().enumerate() {
            let key = (names[&meal.food_id].clone(), meal.eaten_at);
            if !records.contains_key(&key) || !meal.deleted {
                records.insert(key, index);
            }
        }
        // Later entries in one import win too, without creating intermediate
        // acceptance events for duplicate rows.
        let incoming_records: BTreeMap<_, _> = incoming
            .records
            .into_iter()
            .map(|record| ((name_key(&record.name), record.eaten_at), record))
            .collect();
        for ((name, eaten_at), record) in incoming_records {
            let key = (name.clone(), eaten_at);
            let food_id = foods[&name].clone();
            let index = if let Some(&index) = records.get(&key) {
                index
            } else {
                let index = self.meals.len();
                self.meals.push(Meal {
                    id: id(),
                    food_id: food_id.clone(),
                    eaten_at,
                    feedback_id: None,
                    deleted: false,
                });
                records.insert(key, index);
                index
            };
            let previous = self.meals[index].feedback_id.take();
            if let Some(previous) = &previous
                && let Some(event) = self.feedback.iter_mut().find(|f| &f.id == previous)
            {
                event.revoked = true;
            }
            let feedback_id = if let Some(shown_at) = record.shown_at {
                let answered_at = record.answered_at.unwrap_or(eaten_at.max(shown_at));
                if let Some(event) = self.feedback.iter_mut().find(|f| {
                    f.eat
                        && f.food_id == food_id
                        && f.shown_at == shown_at
                        && f.answered_at == answered_at
                }) {
                    event.revoked = false;
                    Some(event.id.clone())
                } else {
                    let event_id = id();
                    self.feedback.push(Feedback {
                        id: event_id.clone(),
                        food_id: food_id.clone(),
                        shown_at,
                        answered_at,
                        eat: true,
                        revoked: false,
                        shown_probability: 0.5,
                        selection_probability: 1.0,
                    });
                    Some(event_id)
                }
            } else {
                None
            };
            let meal = &mut self.meals[index];
            meal.food_id = food_id;
            meal.eaten_at = eaten_at;
            meal.feedback_id = feedback_id;
            meal.deleted = false;
        }
        let mut feedback: HashMap<_, _> = self
            .feedback
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.eat)
            .map(|(index, f)| {
                (
                    (names[&f.food_id].clone(), f.shown_at, f.answered_at),
                    index,
                )
            })
            .collect();
        for rejection in incoming.rejected {
            let name = name_key(&rejection.name);
            let key = (name.clone(), rejection.shown_at, rejection.answered_at);
            if let Some(&index) = feedback.get(&key) {
                self.feedback[index].revoked = false;
            } else {
                feedback.insert(key, self.feedback.len());
                self.feedback.push(Feedback {
                    id: id(),
                    food_id: foods[&name].clone(),
                    shown_at: rejection.shown_at,
                    answered_at: rejection.answered_at,
                    eat: false,
                    revoked: false,
                    shown_probability: 0.5,
                    selection_probability: 1.0,
                });
            }
        }
        Ok(())
    }
}
