//! DuckDB access: one async writer task, and a small pool of reader
//! connections shared by the REST API. All blocking DuckDB work is dispatched
//! to tokio's blocking pool.

use duckdb::{Connection, Transaction};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::rows::{self, LogRow, MetricPointRow, SpanRow, SCHEMA_SQL};

type Ack = oneshot::Sender<Result<(), String>>;

enum DbJob {
    Spans(Vec<SpanRow>),
    Logs(Vec<LogRow>),
    Metrics(Vec<MetricPointRow>),
    Reset,
}

pub struct Db {
    /// Keeps the underlying database instance alive (matters for :memory:).
    /// Only touched to spawn extra reader connections.
    master: Mutex<Connection>,
    writer: mpsc::UnboundedSender<(DbJob, Ack)>,
    readers: Mutex<Vec<Connection>>,
    db_file: Option<String>,
}

fn es(e: duckdb::Error) -> String {
    e.to_string()
}

impl Db {
    pub async fn new(file: Option<&str>) -> anyhow::Result<Self> {
        let file_owned = file.map(str::to_string);
        let master = tokio::task::spawn_blocking(move || -> anyhow::Result<Connection> {
            let conn = match file_owned.as_deref() {
                Some(f) => Connection::open(f)?,
                None => Connection::open_in_memory()?,
            };
            conn.execute_batch(SCHEMA_SQL)?;
            conn.execute_batch("SET threads TO 4;")?;
            Ok(conn)
        })
        .await??;

        let writer = master.try_clone()?;
        let readers = vec![master.try_clone()?, master.try_clone()?];
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(writer_task(writer, rx));

        Ok(Self {
            master: Mutex::new(master),
            writer: tx,
            readers: Mutex::new(readers),
            db_file: file.map(str::to_string),
        })
    }

    pub fn db_file(&self) -> Option<&str> {
        self.db_file.as_deref()
    }

    pub async fn insert_spans(&self, rows: Vec<SpanRow>) -> Result<(), String> {
        self.send_job(DbJob::Spans(rows)).await
    }

    pub async fn insert_logs(&self, rows: Vec<LogRow>) -> Result<(), String> {
        self.send_job(DbJob::Logs(rows)).await
    }

    pub async fn insert_metrics(&self, rows: Vec<MetricPointRow>) -> Result<(), String> {
        self.send_job(DbJob::Metrics(rows)).await
    }

    /// Delete all telemetry rows (spans, logs, metric points).
    pub async fn reset(&self) -> Result<(), String> {
        self.send_job(DbJob::Reset).await
    }

    async fn send_job(&self, job: DbJob) -> Result<(), String> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.writer
            .send((job, ack_tx))
            .map_err(|_| "writer task stopped".to_string())?;
        match ack_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("writer task dropped the job".to_string()),
        }
    }

    /// Run a read-only closure on a pooled connection, off the async runtime.
    pub async fn read<T, F>(&self, f: F) -> anyhow::Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> anyhow::Result<T> + Send + 'static,
    {
        let conn = self.acquire_reader().await?;
        let res = tokio::task::spawn_blocking(move || {
            let out = f(&conn);
            (out, conn)
        })
        .await
        .map_err(|e| anyhow::anyhow!("reader task panicked: {e}"))?;
        let (out, conn) = res;
        self.readers.lock().await.push(conn);
        out
    }

    async fn acquire_reader(&self) -> anyhow::Result<Connection> {
        if let Some(c) = self.readers.lock().await.pop() {
            return Ok(c);
        }
        let master = self.master.lock().await;
        Ok(master.try_clone()?)
    }
}

async fn writer_task(mut conn: Connection, mut rx: mpsc::UnboundedReceiver<(DbJob, Ack)>) {
    while let Some(item) = rx.recv().await {
        let res = tokio::task::spawn_blocking(move || {
            run_job(&mut conn, item);
            conn
        })
        .await;
        match res {
            Ok(c) => conn = c,
            Err(e) => {
                tracing::error!(error = %e, "writer task panicked; stopping");
                return;
            }
        }
    }
}

fn run_job(conn: &mut Connection, (job, ack): (DbJob, Ack)) {
    let result: Result<(), String> = (|| {
        let tx: Transaction = conn.transaction().map_err(es)?;
        match &job {
            DbJob::Spans(rows) => rows::insert_spans(&tx, rows).map_err(es)?,
            DbJob::Logs(rows) => rows::insert_logs(&tx, rows).map_err(es)?,
            DbJob::Metrics(rows) => rows::insert_metrics(&tx, rows).map_err(es)?,
            DbJob::Reset => tx
                .execute_batch("DELETE FROM spans; DELETE FROM log_records; DELETE FROM metric_points;")
                .map_err(es)?,
        }
        tx.commit().map_err(es)
    })();
    if let Err(e) = &result {
        tracing::error!(error = %e, "duckdb write failed");
    }
    let _ = ack.send(result);
}
