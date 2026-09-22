use crate::model::{Model, POLICY};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

pub type Timestamp = i64;
pub const DAY: f64 = 86_400_000.0;
pub const MINUTE: i64 = 60_000;
pub fn minute(time: Timestamp) -> Timestamp {
    time.div_euclid(MINUTE) * MINUTE
}
pub const MAX_BACKUP_BYTES: usize = 16 * 1024 * 1024;
const CARD_LIFETIME: i64 = 2 * 60 * 60 * 1000;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(String),
    #[error("本地数据库操作失败：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("文件操作失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("备份格式无法读取：{0}")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, AppError>;
fn invalid(message: &str) -> AppError {
    AppError::Invalid(message.into())
}
pub fn id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Food {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub deleted: bool,
    pub model: Model,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Meal {
    pub id: String,
    pub food_id: String,
    pub eaten_at: Timestamp,
    pub feedback_id: Option<String>,
    pub deleted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Feedback {
    pub id: String,
    pub food_id: String,
    pub shown_at: Timestamp,
    pub answered_at: Timestamp,
    pub eat: bool,
    pub revoked: bool,
    pub shown_probability: f64,
    pub selection_probability: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub food_id: String,
    pub shown_at: Timestamp,
    pub last_eaten: Option<Timestamp>,
    pub probability: f64,
    pub selection_probability: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Decision {
    Ready {
        seen: Vec<String>,
        candidate: Candidate,
    },
    Accepted {
        meal_id: String,
        food_id: String,
    },
    Exhausted,
    Idle,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub format_version: u32,
    pub policy: String,
    pub revision: u64,
    pub foods: Vec<Food>,
    pub meals: Vec<Meal>,
    pub feedback: Vec<Feedback>,
    pub decision: Decision,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            format_version: 1,
            policy: POLICY.into(),
            revision: 0,
            foods: Vec::new(),
            meals: Vec::new(),
            feedback: Vec::new(),
            decision: Decision::Idle,
        }
    }
}

impl Data {
    pub fn food(&self, food_id: &str) -> Option<&Food> {
        self.foods.iter().find(|f| f.id == food_id)
    }
    pub fn last_eaten(&self, food_id: &str, before: Timestamp) -> Option<Timestamp> {
        self.meals
            .iter()
            .filter(|m| !m.deleted && m.food_id == food_id && m.eaten_at <= before)
            .map(|m| m.eaten_at)
            .max()
    }
    pub fn eaten_count(&self) -> usize {
        self.meals.iter().filter(|m| !m.deleted).count()
    }

    pub fn add_food(&mut self, name: &str) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 32 {
            return Err(invalid("名称需要 1–32 个字。"));
        }
        if self
            .foods
            .iter()
            .any(|f| !f.deleted && f.name.to_lowercase() == name.to_lowercase())
        {
            return Err(invalid("这个选项已经存在，可以在列表中重新启用。"));
        }
        if self.foods.len() >= 500 {
            return Err(invalid("最多保存 500 个用餐选项。"));
        }
        let key = id();
        self.foods.push(Food {
            id: key.clone(),
            name: name.into(),
            enabled: true,
            deleted: false,
            model: Model::default(),
        });
        self.invalidate_card();
        Ok(key)
    }

    pub fn menu(&self) -> impl Iterator<Item = &Food> {
        self.foods.iter().filter(|f| !f.deleted)
    }

    pub fn rename_food(&mut self, food_id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 32 {
            return Err(invalid("名称需要 1–32 个字。"));
        }
        if self
            .menu()
            .any(|f| f.id != food_id && f.name.to_lowercase() == name.to_lowercase())
        {
            return Err(invalid("这个名称已经在菜单里了。"));
        }
        self.foods
            .iter_mut()
            .find(|f| f.id == food_id && !f.deleted)
            .ok_or_else(|| invalid("找不到这个菜单选项。"))?
            .name = name.into();
        Ok(())
    }

    pub fn delete_food(&mut self, food_id: &str) -> Result<()> {
        let food = self
            .foods
            .iter_mut()
            .find(|f| f.id == food_id)
            .ok_or_else(|| invalid("找不到这个菜单选项。"))?;
        if food.deleted {
            return Ok(());
        }
        food.deleted = true;
        food.enabled = false;
        // Keep the identity for meal history and feedback replay, but never
        // leave a pending or accepted card referring to a removed menu item.
        self.decision = Decision::Idle;
        Ok(())
    }

    pub fn set_enabled(&mut self, food_id: &str, enabled: bool) -> Result<()> {
        self.foods
            .iter_mut()
            .find(|f| f.id == food_id && !f.deleted)
            .ok_or_else(|| invalid("找不到这个选项。"))?
            .enabled = enabled;
        self.invalidate_card();
        Ok(())
    }

    pub fn invalidate_card(&mut self) {
        if matches!(self.decision, Decision::Ready { .. }) {
            self.decision = Decision::Idle;
        }
    }

    pub fn start(&mut self, now: Timestamp, draw: f64, new_round: bool) -> Result<()> {
        if !draw.is_finite() || !(0.0..1.0).contains(&draw) {
            return Err(invalid("抽样值无效。"));
        }
        if !new_round {
            match &self.decision {
                Decision::Ready { candidate, .. }
                    if now >= candidate.shown_at && now - candidate.shown_at < CARD_LIFETIME =>
                {
                    return Ok(());
                }
                Decision::Accepted { .. } | Decision::Exhausted => return Ok(()),
                _ => {}
            }
        }
        self.choose(now, draw, Vec::new());
        Ok(())
    }

    fn choose(&mut self, now: Timestamp, draw: f64, seen: Vec<String>) {
        let candidates: Vec<_> = self
            .foods
            .iter()
            .filter(|f| !f.deleted && f.enabled && !seen.contains(&f.id))
            .map(|food| {
                let last = self.last_eaten(&food.id, now);
                let probability = last
                    .map(|t| food.model.predict((now - t) as f64 / DAY))
                    .unwrap_or(0.5);
                (food.id.clone(), last, probability)
            })
            .collect();
        if candidates.is_empty() {
            self.decision = Decision::Exhausted;
            return;
        }
        let sum: f64 = candidates.iter().map(|c| c.2.powi(2)).sum();
        let mut cumulative = 0.0;
        let count = candidates.len();
        for (index, (food_id, last_eaten, probability)) in candidates.into_iter().enumerate() {
            let selection_probability = 0.9 * probability.powi(2) / sum + 0.1 / count as f64;
            cumulative += selection_probability;
            if draw < cumulative || index == count - 1 {
                self.decision = Decision::Ready {
                    seen,
                    candidate: Candidate {
                        id: id(),
                        food_id,
                        shown_at: now,
                        last_eaten,
                        probability,
                        selection_probability,
                    },
                };
                return;
            }
        }
    }

    pub fn answer(
        &mut self,
        recommendation_id: &str,
        eat: bool,
        now: Timestamp,
        draw: f64,
    ) -> Result<()> {
        // A retry cannot add a second sample, including after undo.
        if let Some(previous) = self.feedback.iter().find(|f| f.id == recommendation_id) {
            if previous.eat != eat {
                return Err(invalid("这张卡片已经回答过了，请刷新当前状态。"));
            }
            return Ok(());
        }
        let Decision::Ready {
            mut seen,
            candidate,
        } = self.decision.clone()
        else {
            return Err(invalid("这一轮已经结束，请重新开始。"));
        };
        if candidate.id != recommendation_id {
            return Err(invalid("卡片已更新，请使用当前候选。"));
        }
        if now < candidate.shown_at || now - candidate.shown_at >= CARD_LIFETIME {
            return Err(invalid("这张卡片已过期，请开始新的一轮。"));
        }
        if !draw.is_finite() || !(0.0..1.0).contains(&draw) {
            return Err(invalid("抽样值无效。"));
        }
        let feedback_id = candidate.id.clone();
        self.feedback.push(Feedback {
            id: feedback_id.clone(),
            food_id: candidate.food_id.clone(),
            shown_at: candidate.shown_at,
            answered_at: now,
            eat,
            revoked: false,
            shown_probability: candidate.probability,
            selection_probability: candidate.selection_probability,
        });
        if eat {
            let meal_id = id();
            self.meals.push(Meal {
                id: meal_id.clone(),
                food_id: candidate.food_id.clone(),
                eaten_at: minute(now),
                feedback_id: Some(feedback_id),
                deleted: false,
            });
            self.decision = Decision::Accepted {
                meal_id,
                food_id: candidate.food_id,
            };
            self.normalize_meal_minutes();
            self.rebuild();
        } else {
            seen.push(candidate.food_id);
            self.rebuild();
            self.choose(now, draw, seen);
        }
        Ok(())
    }

    pub fn save_meal(
        &mut self,
        meal_id: Option<&str>,
        food_id: &str,
        eaten_at: Timestamp,
        now: Timestamp,
    ) -> Result<()> {
        let option = self
            .food(food_id)
            .ok_or_else(|| invalid("请先选择食物。"))?;
        if option.deleted
            && !self
                .meals
                .iter()
                .any(|m| Some(m.id.as_str()) == meal_id && !m.deleted && m.food_id == food_id)
        {
            return Err(invalid("这个选项已从菜单删除，请选择其他食物。"));
        }
        if eaten_at < 0 || eaten_at > now {
            return Err(invalid("食用时间不能是未来。"));
        }
        let eaten_at = minute(eaten_at);
        if let Some(key) = meal_id {
            let meal = self
                .meals
                .iter_mut()
                .find(|m| m.id == key && !m.deleted)
                .ok_or_else(|| invalid("这条记录已不存在。"))?;
            if let Some(key) = meal.feedback_id.take()
                && let Some(f) = self.feedback.iter_mut().find(|f| f.id == key)
            {
                f.revoked = true;
            }
            meal.food_id = food_id.into();
            meal.eaten_at = eaten_at;
        } else {
            self.meals.push(Meal {
                id: id(),
                food_id: food_id.into(),
                eaten_at,
                feedback_id: None,
                deleted: false,
            });
        }
        self.decision = Decision::Idle;
        self.normalize_meal_minutes();
        self.rebuild();
        Ok(())
    }

    pub fn remove_meal(&mut self, meal_id: &str) -> Result<()> {
        let meal = self
            .meals
            .iter_mut()
            .find(|m| m.id == meal_id)
            .ok_or_else(|| invalid("找不到这条记录。"))?;
        if meal.deleted {
            return Ok(());
        }
        meal.deleted = true;
        if let Some(key) = &meal.feedback_id
            && let Some(f) = self.feedback.iter_mut().find(|f| &f.id == key)
        {
            f.revoked = true;
        }
        self.decision = Decision::Idle;
        self.rebuild();
        Ok(())
    }

    /// Minute precision is a storage invariant. Keep the last live record for
    /// a name/minute collision and revoke the replaced acceptance sample.
    pub fn normalize_meal_minutes(&mut self) {
        let names: std::collections::HashMap<_, _> = self
            .foods
            .iter()
            .map(|f| (f.id.clone(), f.name.trim().to_lowercase()))
            .collect();
        let mut records = std::collections::HashMap::new();
        let mut replaced = Vec::new();
        for (index, meal) in self.meals.iter_mut().enumerate() {
            meal.eaten_at = minute(meal.eaten_at);
            if !meal.deleted
                && let Some(previous) =
                    records.insert((names[&meal.food_id].clone(), meal.eaten_at), index)
            {
                replaced.push(previous);
            }
        }
        for index in replaced {
            let meal = &mut self.meals[index];
            meal.deleted = true;
            if let Some(key) = &meal.feedback_id
                && let Some(feedback) = self.feedback.iter_mut().find(|f| &f.id == key)
            {
                feedback.revoked = true;
            }
        }
        if let Decision::Accepted { meal_id, .. } = &self.decision
            && self.meals.iter().any(|m| &m.id == meal_id && m.deleted)
        {
            self.decision = Decision::Idle;
        }
    }

    /// Canonical replay: actual meals are facts, models are derived. The accepted
    /// meal belonging to this feedback must never become its own previous meal.
    pub fn rebuild(&mut self) {
        // Imports can arrive out of order; always replay facts chronologically.
        self.feedback
            .sort_by_key(|f| (f.shown_at, f.answered_at, f.eat));
        let accepted_times: std::collections::HashMap<_, _> = self
            .feedback
            .iter()
            .filter(|f| f.eat)
            .map(|f| (f.id.as_str(), f.answered_at))
            .collect();
        let effective_time = |meal: &Meal| {
            meal.feedback_id
                .as_deref()
                .and_then(|id| accepted_times.get(id))
                .copied()
                .unwrap_or(meal.eaten_at)
        };
        for food in &mut self.foods {
            let mut model = Model::default();
            let mut meals: Vec<_> = self
                .meals
                .iter()
                .filter(|m| !m.deleted && m.food_id == food.id)
                .collect();
            meals.sort_by_key(|m| effective_time(m));
            for f in self
                .feedback
                .iter()
                .filter(|f| !f.revoked && f.food_id == food.id)
            {
                let end = meals.partition_point(|m| effective_time(m) <= f.shown_at);
                let previous = meals[..end]
                    .iter()
                    .rev()
                    .find(|m| m.feedback_id.as_deref() != Some(&f.id))
                    .map(|m| m.eaten_at);
                if let Some(time) = previous {
                    model.update((f.shown_at - time) as f64 / DAY, f.eat);
                }
            }
            food.model = model;
        }
    }

    /// Read legacy snapshots without retaining the unused profile/session IDs.
    pub(crate) fn from_snapshot(text: &str) -> Result<Self> {
        let mut value: serde_json::Value = serde_json::from_str(text)?;
        if let Some(object) = value.as_object_mut() {
            object.remove("profile_id");
        }
        if let Some(ready) = value
            .pointer_mut("/decision/Ready")
            .and_then(|v| v.as_object_mut())
        {
            ready.remove("session_id");
        }
        // A feedback event uses its recommendation ID directly. Re-map old
        // meal links before dropping the redundant second event identifier.
        let mut links = std::collections::HashMap::new();
        if let Some(feedback) = value.get_mut("feedback").and_then(|v| v.as_array_mut()) {
            for event in feedback {
                if let Some(event) = event.as_object_mut()
                    && let Some(recommendation) = event.remove("recommendation_id")
                {
                    if let (Some(old), Some(new)) = (
                        event.get("id").and_then(|v| v.as_str()),
                        recommendation.as_str(),
                    ) {
                        links.insert(old.to_string(), new.to_string());
                    }
                    event.insert("id".into(), recommendation);
                }
            }
        }
        if let Some(meals) = value.get_mut("meals").and_then(|v| v.as_array_mut()) {
            for meal in meals {
                if let Some(new) = meal
                    .get("feedback_id")
                    .and_then(|v| v.as_str())
                    .and_then(|old| links.get(old))
                {
                    meal["feedback_id"] = new.clone().into();
                }
            }
        }
        Ok(serde_json::from_value(value)?)
    }

    /// Internal recovery snapshot. User-facing exchange uses export_text().
    pub fn backup(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn from_backup(text: &str, now: Timestamp) -> Result<Self> {
        if text.len() > MAX_BACKUP_BYTES {
            return Err(invalid("备份不能超过 16 MB。"));
        }
        let mut data = Self::from_snapshot(text)?;
        data.validate(now)?;
        data.decision = Decision::Idle;
        data.normalize_meal_minutes();
        data.rebuild();
        Ok(data)
    }

    pub fn validate(&self, now: Timestamp) -> Result<()> {
        if self.format_version != 1 || self.policy != POLICY {
            return Err(invalid("备份版本与当前应用不兼容。"));
        }
        if self.foods.len() > 500 || self.meals.len() > 50_000 || self.feedback.len() > 50_000 {
            return Err(invalid("备份记录数量超出当前版本支持范围。"));
        }
        let food_ids: HashSet<_> = self.foods.iter().map(|f| f.id.as_str()).collect();
        let meal_ids: HashSet<_> = self.meals.iter().map(|m| m.id.as_str()).collect();
        let feedback_ids: HashSet<_> = self.feedback.iter().map(|f| f.id.as_str()).collect();
        if food_ids.len() != self.foods.len()
            || meal_ids.len() != self.meals.len()
            || feedback_ids.len() != self.feedback.len()
        {
            return Err(invalid("备份包含重复标识。"));
        }
        let mut names = HashSet::new();
        for food in &self.foods {
            if Uuid::parse_str(&food.id).is_err()
                || food.name.trim().is_empty()
                || food.name.chars().count() > 32
                || (food.deleted && food.enabled)
                || (!food.deleted && !names.insert(food.name.trim().to_lowercase()))
            {
                return Err(invalid("备份包含无效或重复的食物。"));
            }
        }
        for m in &self.meals {
            if !food_ids.contains(m.food_id.as_str())
                || Uuid::parse_str(&m.id).is_err()
                || m.eaten_at < 0
                || m.eaten_at > now
            {
                return Err(invalid("备份包含无效的食用记录。"));
            }
            if let Some(key) = &m.feedback_id {
                let f = self
                    .feedback
                    .iter()
                    .find(|f| &f.id == key)
                    .ok_or_else(|| invalid("备份中的反馈关联已损坏。"))?;
                if !f.eat || f.food_id != m.food_id || (!m.deleted && f.revoked) {
                    return Err(invalid("备份中的食用与反馈不一致。"));
                }
            }
        }
        for f in &self.feedback {
            if !food_ids.contains(f.food_id.as_str())
                || Uuid::parse_str(&f.id).is_err()
                || f.shown_at < 0
                || f.answered_at < f.shown_at
                || f.answered_at > now
                || !(0.0..=1.0).contains(&f.shown_probability)
                || !(0.0..=1.0).contains(&f.selection_probability)
            {
                return Err(invalid("备份包含无效的反馈。"));
            }
            if f.eat
                && !f.revoked
                && self
                    .meals
                    .iter()
                    .filter(|m| !m.deleted && m.feedback_id.as_deref() == Some(&f.id))
                    .count()
                    != 1
            {
                return Err(invalid("备份中的接受反馈缺少唯一食用记录。"));
            }
        }
        Ok(())
    }
}
