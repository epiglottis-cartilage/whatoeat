use crate::domain::*;
use crate::theme::Theme;
use rand::Rng;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Store {
    path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    AddFood {
        name: String,
    },
    AddStarters,
    AddEnglishStarters,
    RenameFood {
        food_id: String,
        name: String,
    },
    DeleteFood {
        food_id: String,
    },
    SetEnabled {
        food_id: String,
        enabled: bool,
    },
    Start {
        new_round: bool,
    },
    Answer {
        recommendation_id: String,
        eat: bool,
    },
    SaveMeal {
        meal_id: Option<String>,
        food_id: String,
        eaten_at: Timestamp,
    },
    RemoveMeal {
        meal_id: String,
    },
    Clear,
    Merge {
        text: String,
    },
    Restore {
        backup: String,
    },
}

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(&self.path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 3 {
            return Err(AppError::Invalid(
                "数据库来自更新的应用版本，请先升级应用。".into(),
            ));
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS app_state (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS command_receipts (request_id TEXT PRIMARY KEY, command TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        // Serialize migrations across windows and keep the original payload
        // before removing legacy fields. A failed migration leaves the original version intact.
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 3 {
            let old: Option<String> = tx
                .query_row("SELECT payload FROM app_state WHERE id=1", [], |r| r.get(0))
                .optional()?;
            if let Some(old) = old {
                let mut state = Data::from_snapshot(&old)?;
                state.validate(i64::MAX)?;
                self.save_recovery_text(&old, chrono::Utc::now().timestamp_millis(), "upgrade")?;
                state.normalize_meal_minutes();
                state.rebuild();
                tx.execute(
                    "UPDATE app_state SET payload=?1 WHERE id=1",
                    [state.backup()?],
                )?;
            }
            tx.pragma_update(None, "user_version", 3)?;
        }
        tx.commit()?;
        Ok(conn)
    }
    fn read(conn: &Connection) -> Result<Data> {
        let json: Option<String> = conn
            .query_row("SELECT payload FROM app_state WHERE id=1", [], |r| r.get(0))
            .optional()?;
        let state = json
            .map(|s| Data::from_snapshot(&s))
            .transpose()?
            .unwrap_or_default();
        if state.format_version != 1 || state.policy != crate::model::POLICY {
            return Err(AppError::Invalid("数据库或模型版本不兼容。".into()));
        }
        Ok(state)
    }
    pub fn load(&self) -> Result<Data> {
        let conn = self.connect()?;
        Self::read(&conn)
    }
    pub fn load_theme(&self) -> Result<Theme> {
        let conn = self.connect()?;
        let key: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key='theme'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(key.as_deref().and_then(Theme::from_key).unwrap_or_default())
    }
    pub fn save_theme(&self, theme: Theme) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO app_settings(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [theme.key()],
        )?;
        Ok(())
    }
    pub fn execute(&self, request_id: &str, command: Command, now: Timestamp) -> Result<Data> {
        let mut conn = self.connect()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let serialized = serde_json::to_string(&command)?;
        if let Some(previous) = tx
            .query_row(
                "SELECT command FROM command_receipts WHERE request_id=?1",
                [request_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            if previous != serialized {
                return Err(AppError::Invalid("同一请求标识不能用于不同操作。".into()));
            }
            return Self::read(&tx);
        }
        let mut state = Self::read(&tx)?;
        let draw = rand::rng().random::<f64>();
        match command {
            Command::AddFood { name } => {
                state.add_food(&name)?;
            }
            starter @ (Command::AddStarters | Command::AddEnglishStarters) => {
                let names = if matches!(starter, Command::AddEnglishStarters) {
                    [
                        "Beef noodles",
                        "Mala hot pot",
                        "Curry rice",
                        "Dumplings",
                        "Sushi",
                        "Roast meat rice",
                    ]
                } else {
                    ["牛肉面", "麻辣烫", "咖喱饭", "饺子", "寿司", "烧腊饭"]
                };
                for name in names {
                    if !state.menu().any(|f| f.name == name) {
                        state.add_food(name)?;
                    }
                }
            }
            Command::RenameFood { food_id, name } => state.rename_food(&food_id, &name)?,
            Command::DeleteFood { food_id } => state.delete_food(&food_id)?,
            Command::SetEnabled { food_id, enabled } => state.set_enabled(&food_id, enabled)?,
            Command::Start { new_round } => state.start(now, draw, new_round)?,
            Command::Answer {
                recommendation_id,
                eat,
            } => state.answer(&recommendation_id, eat, now, draw)?,
            Command::SaveMeal {
                meal_id,
                food_id,
                eaten_at,
            } => state.save_meal(meal_id.as_deref(), &food_id, eaten_at, now)?,
            Command::RemoveMeal { meal_id } => state.remove_meal(&meal_id)?,
            Command::Clear => {
                // Reset all persisted facts and old request payloads together.
                let revision = state.revision;
                state = Data::default();
                state.revision = revision;
                tx.execute("DELETE FROM command_receipts", [])?;
            }
            Command::Merge { text } => {
                state.merge_text(&text, now)?;
                self.save_recovery(&Self::read(&tx)?, now)?;
            }
            Command::Restore { backup } => {
                let mut restored = Data::from_backup(&backup, now)?;
                // The old state is recoverable even after replacing the profile.
                self.save_recovery(&state, now)?;
                restored.revision = state.revision;
                state = restored;
            }
        }
        state.revision += 1;
        if state.meals.len() > 50_000 || state.feedback.len() > 50_000 {
            return Err(AppError::Invalid(
                "记录已达到当前版本的容量上限，请先导出备份。".into(),
            ));
        }
        // Bound the export representation too: every committed state must be
        // small enough to restore through the same backup interface.
        let payload = state.backup()?;
        if payload.len() > MAX_BACKUP_BYTES {
            return Err(AppError::Invalid(
                "数据超出当前版本的容量限制，请先导出备份。".into(),
            ));
        }
        tx.execute("INSERT INTO app_state(id,payload) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload", [payload])?;
        tx.execute(
            "INSERT INTO command_receipts(request_id,command) VALUES(?1,?2)",
            params![request_id, serialized],
        )?;
        tx.commit()?;
        Ok(state)
    }
    fn save_recovery(&self, data: &Data, now: Timestamp) -> Result<()> {
        self.save_recovery_text(&data.backup()?, now, "import")
    }

    fn save_recovery_text(&self, text: &str, now: Timestamp, reason: &str) -> Result<()> {
        use std::io::Write;
        let dir = self
            .path
            .parent()
            .unwrap_or(Path::new("."))
            .join("recovery");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("before-{reason}-{now}-{}.json", id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}
