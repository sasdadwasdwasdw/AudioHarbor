use crate::{
    error::{AppError, Result},
    models::{Job, Settings, Status},
};
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};

pub struct Storage(Mutex<Connection>);
impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS jobs (id TEXT PRIMARY KEY, data TEXT NOT NULL); PRAGMA user_version=1;")?;
        Ok(Self(Mutex::new(db)))
    }
    pub fn settings(&self, default: Settings) -> Result<Settings> {
        use rusqlite::OptionalExtension;
        let db = self
            .0
            .lock()
            .map_err(|_| AppError::new("storage", "Database lock unavailable."))?;
        let value: Option<String> = db
            .query_row("SELECT data FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()?;
        match value {
            Some(v) => Ok(serde_json::from_str(&v)?),
            None => Ok(default),
        }
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.0.lock().map_err(|_|AppError::new("storage","Database lock unavailable."))?.execute("INSERT INTO settings(id,data) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",[serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn save_jobs(&self, jobs: &[Job]) -> Result<()> {
        let mut db = self
            .0
            .lock()
            .map_err(|_| AppError::new("storage", "Database lock unavailable."))?;
        let tx = db.transaction()?;
        for job in jobs {
            tx.execute("INSERT INTO jobs(id,data) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![job.id,serde_json::to_string(job)?])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn jobs(&self) -> Result<Vec<Job>> {
        let db = self
            .0
            .lock()
            .map_err(|_| AppError::new("storage", "Database lock unavailable."))?;
        let mut stmt = db.prepare("SELECT data FROM jobs ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut jobs = Vec::new();
        for row in rows {
            let mut job: Job = serde_json::from_str(&row?)?;
            if job.status.active() || job.status == Status::Queued {
                job.status = Status::Interrupted;
                job.error=Some("The app closed before this job finished. Retry to reuse any supported partial data.".into());
            }
            jobs.push(job);
        }
        Ok(jobs)
    }
    pub fn remove(&self, ids: &[String]) -> Result<()> {
        let mut db = self
            .0
            .lock()
            .map_err(|_| AppError::new("storage", "Database lock unavailable."))?;
        let tx = db.transaction()?;
        for id in ids {
            tx.execute("DELETE FROM jobs WHERE id=?1", [id])?;
        }
        tx.commit()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("data.db");
        let db = Storage::open(&p).unwrap();
        let mut settings = Settings::defaults("C:\\เพลง".into());
        settings.concurrency = 3;
        settings.language = "th".into();
        db.save_settings(&settings).unwrap();
        drop(db);
        let db = Storage::open(&p).unwrap();
        let loaded = db.settings(Settings::defaults("ignored".into())).unwrap();
        assert_eq!(loaded.output_dir, settings.output_dir);
        assert_eq!(loaded.concurrency, 3);
        assert_eq!(loaded.language, "th");
    }
}
