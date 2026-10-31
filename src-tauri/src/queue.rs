use crate::{
    download::{self, Tick},
    error::{AppError, Result},
    models::*,
    process::Engines,
    storage::Storage,
};
use std::{collections::HashMap, sync::Arc};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

pub struct Inner {
    pub jobs: Vec<Job>,
    pub settings: Settings,
    pub active: HashMap<String, CancellationToken>,
    pub maintenance: bool,
}
impl Inner {
    fn next_index(&self) -> Option<usize> {
        if self.maintenance || self.active.len() >= self.settings.concurrency {
            None
        } else {
            self.jobs.iter().position(|j| j.status == Status::Queued)
        }
    }
}
pub struct Queue {
    pub inner: Mutex<Inner>,
    pub storage: Storage,
    pub notify: Notify,
    pub engines: Engines,
    pub app: AppHandle,
}
impl Queue {
    pub fn new(
        app: AppHandle,
        storage: Storage,
        settings: Settings,
        jobs: Vec<Job>,
        engines: Engines,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                jobs,
                settings,
                active: HashMap::new(),
                maintenance: false,
            }),
            storage,
            notify: Notify::new(),
            engines,
            app,
        })
    }
    pub async fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock().await;
        Snapshot {
            jobs: inner.jobs.clone(),
            settings: inner.settings.clone(),
        }
    }
    fn emit(&self, job: &Job) {
        if let Err(e) = self.app.emit("job-changed", job) {
            log::warn!("Could not emit job event: {e}");
        }
    }
    pub async fn enqueue(
        &self,
        items: Vec<Media>,
        format: AudioFormat,
        quality: u16,
    ) -> Result<Vec<Job>> {
        if items.is_empty() || items.len() > 100 || ![128, 192, 256, 320].contains(&quality) {
            return Err(AppError::new(
                "queue",
                "Choose 1–100 entries and a valid bitrate.",
            ));
        }
        self.engines.verify()?;
        let mut inner = self.inner.lock().await;
        if inner.maintenance {
            return Err(AppError::new(
                "updating",
                "An update is being installed. Try again after restarting.",
            ));
        }
        if inner
            .jobs
            .iter()
            .filter(|j| j.status == Status::Queued || j.status.active())
            .count()
            + items.len()
            > 200
        {
            return Err(AppError::new("queue", "The queue is full (200 jobs)."));
        }
        crate::paths::validate_directory(&inner.settings.output_dir)?;
        let now = chrono::Utc::now().to_rfc3339();
        let jobs = items
            .into_iter()
            .map(|mut media| {
                media.url = crate::paths::validate_url(&media.url)?;
                Ok(Job {
                    id: uuid::Uuid::new_v4().to_string(),
                    media,
                    status: Status::Queued,
                    format: format.clone(),
                    quality,
                    output_dir: inner.settings.output_dir.clone(),
                    output_file: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    progress: Progress::default(),
                    error: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.storage.save_jobs(&jobs)?;
        inner.jobs.extend(jobs.clone());
        for job in &jobs {
            self.emit(job);
        }
        drop(inner);
        self.notify.notify_one();
        Ok(jobs)
    }
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        if let Some(token) = inner.active.get(id) {
            token.cancel();
            return Ok(());
        }
        let job = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| AppError::new("job", "Job not found."))?;
        if job.status == Status::Queued {
            let mut next = job.clone();
            next.status = Status::Cancelled;
            next.updated_at = chrono::Utc::now().to_rfc3339();
            self.storage.save_jobs(&[next.clone()])?;
            *job = next;
            self.emit(job);
        }
        Ok(())
    }
    pub async fn retry(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().await;
        if inner.maintenance {
            return Err(AppError::new("updating", "An update is being installed."));
        }
        let job = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| AppError::new("job", "Job not found."))?;
        if !job.status.retryable() {
            return Err(AppError::new(
                "state",
                "Only failed, cancelled, or interrupted jobs can be retried.",
            ));
        }
        let mut next = job.clone();
        next.status = Status::Queued;
        next.error = None;
        next.progress = Progress::default();
        next.updated_at = chrono::Utc::now().to_rfc3339();
        self.storage.save_jobs(&[next.clone()])?;
        *job = next;
        self.emit(job);
        drop(inner);
        self.notify.notify_one();
        Ok(())
    }
    pub async fn clear(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        let ids = inner
            .jobs
            .iter()
            .filter(|j| !j.status.active() && j.status != Status::Queued)
            .map(|j| j.id.clone())
            .collect::<Vec<_>>();
        self.storage.remove(&ids)?;
        inner
            .jobs
            .retain(|j| j.status.active() || j.status == Status::Queued);
        let _ = self.app.emit("history-cleared", ());
        Ok(())
    }
    pub async fn save_settings(&self, settings: Settings) -> Result<Settings> {
        settings.validate()?;
        crate::paths::validate_directory(&settings.output_dir)?;
        let mut inner = self.inner.lock().await;
        self.storage.save_settings(&settings)?;
        inner.settings = settings.clone();
        drop(inner);
        self.notify.notify_one();
        Ok(settings)
    }
    pub fn start(self: &Arc<Self>) {
        let queue = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                queue.notify.notified().await;
                loop {
                    let mut inner = queue.inner.lock().await;
                    let Some(index) = inner.next_index() else {
                        break;
                    };
                    let mut job = inner.jobs[index].clone();
                    debug_assert!(job.status.can_transition(&Status::Downloading));
                    job.status = Status::Downloading;
                    job.updated_at = chrono::Utc::now().to_rfc3339();
                    if let Err(error) = queue.storage.save_jobs(&[job.clone()]) {
                        let _ = queue.app.emit("app-error", error);
                        break;
                    }
                    inner.jobs[index] = job.clone();
                    let token = CancellationToken::new();
                    inner.active.insert(job.id.clone(), token.clone());
                    queue.emit(&job);
                    drop(inner);
                    let worker = queue.clone();
                    tauri::async_runtime::spawn(async move {
                        worker.run(job, token).await;
                    });
                }
            }
        });
    }
    async fn run(self: Arc<Self>, job: Job, token: CancellationToken) {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Tick>(32);
        let progress_queue = self.clone();
        let id = job.id.clone();
        let progress_task = tauri::async_runtime::spawn(async move {
            while let Some(tick) = receiver.recv().await {
                let mut inner = progress_queue.inner.lock().await;
                if let Some(job) = inner.jobs.iter_mut().find(|j| j.id == id) {
                    match tick {
                        Tick::Download(p) => job.progress = p,
                        Tick::Convert(p) => {
                            job.status = Status::Converting;
                            job.progress = Progress {
                                percent: p,
                                ..Progress::default()
                            };
                        }
                    }
                    job.updated_at = chrono::Utc::now().to_rfc3339();
                    progress_queue.emit(job);
                }
            }
        });
        let result = download::execute(&self.engines, &job, token, move |tick| {
            let _ = sender.try_send(tick);
        })
        .await;
        let _ = progress_task.await;
        let mut inner = self.inner.lock().await;
        inner.active.remove(&job.id);
        if let Some(current) = inner.jobs.iter_mut().find(|j| j.id == job.id) {
            current.updated_at = chrono::Utc::now().to_rfc3339();
            match result {
                Ok(path) => {
                    current.status = Status::Completed;
                    current.output_file = Some(path);
                    current.progress = Progress {
                        percent: Some(100.0),
                        ..Progress::default()
                    };
                }
                Err(error) => {
                    log::warn!("Job {} ended with code {}", job.id, error.code);
                    current.status = if error.code == "cancelled" {
                        Status::Cancelled
                    } else {
                        Status::Failed
                    };
                    current.error = Some(error.message);
                }
            }
            if let Err(error) = self.storage.save_jobs(std::slice::from_ref(current)) {
                let _ = self.app.emit("app-error", error);
            }
            self.emit(current);
        }
        drop(inner);
        self.notify.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(status: Status) -> Job {
        let now = chrono::Utc::now().to_rfc3339();
        Job {
            id: uuid::Uuid::new_v4().to_string(),
            media: Media {
                id: "test".into(),
                title: "เพลงทดสอบ".into(),
                url: "https://example.com/audio".into(),
                uploader: None,
                duration: None,
                thumbnail: None,
                source: "test".into(),
                abr: None,
            },
            status,
            format: AudioFormat::Mp3,
            quality: 192,
            output_dir: "C:\\AudioHarbor".into(),
            output_file: None,
            created_at: now.clone(),
            updated_at: now,
            progress: Progress::default(),
            error: None,
        }
    }
    #[test]
    fn scheduler_respects_fifo_capacity_and_update_gate() {
        let mut inner = Inner {
            jobs: vec![
                job(Status::Completed),
                job(Status::Queued),
                job(Status::Queued),
            ],
            settings: Settings::defaults("C:\\AudioHarbor".into()),
            active: HashMap::new(),
            maintenance: false,
        };
        assert_eq!(inner.next_index(), Some(1));
        inner.active.insert("one".into(), CancellationToken::new());
        assert_eq!(inner.next_index(), Some(1));
        inner.active.insert("two".into(), CancellationToken::new());
        assert_eq!(inner.next_index(), None);
        inner.settings.concurrency = 1;
        assert_eq!(inner.next_index(), None);
        assert_eq!(inner.active.len(), 2);
        inner.active.clear();
        inner.maintenance = true;
        assert_eq!(inner.next_index(), None);
        inner.maintenance = false;
        inner.jobs[1].status = Status::Cancelled;
        assert_eq!(inner.next_index(), Some(2));
    }
}
