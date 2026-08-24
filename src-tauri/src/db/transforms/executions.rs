use rusqlite::{params, Result};

use super::super::DbState;
use super::{TransformationExecution, TransformationExecutionStart};

impl DbState {
    pub fn begin_transformation_execution(
        &self,
        request: TransformationExecutionStart<'_>,
    ) -> Result<String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transformation_executions
                (target_kind, target_ref, target_revision, source_clip_id,
                 trigger_kind, destination_kind, input_hash, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                request.target_kind,
                request.target_ref,
                request.target_revision,
                request.source_clip_id,
                request.trigger_kind,
                request.destination_kind,
                request.input_hash
            ],
        )?;
        conn.query_row(
            "SELECT id FROM transformation_executions WHERE rowid = last_insert_rowid()",
            [],
            |row| row.get(0),
        )
    }

    pub fn finish_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
        output_hash: Option<&str>,
        error_summary: Option<&str>,
    ) -> Result<()> {
        let status = if error_summary.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = ?2, output_hash = ?3, error_summary = ?4,
                 completed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?5",
            params![
                duration_ms,
                status,
                output_hash,
                error_summary,
                execution_id
            ],
        )?;
        Ok(())
    }

    pub fn cancel_transformation_execution(
        &self,
        execution_id: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions
             SET duration_ms = ?1, status = 'cancelled', output_hash = NULL,
                 error_summary = NULL,
                 completed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?2",
            params![duration_ms, execution_id],
        )?;
        Ok(())
    }

    pub fn start_transformation_execution(&self, execution_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transformation_executions SET status = 'running'
             WHERE id = ?1 AND status = 'queued'",
            params![execution_id],
        )?;
        Ok(())
    }

    pub fn get_clip_transformation_executions(
        &self,
        clip_id: i64,
    ) -> Result<Vec<TransformationExecution>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, target_kind, target_ref, target_revision, source_clip_id,
                    trigger_kind, destination_kind, started_at, completed_at,
                    duration_ms, status, error_summary
             FROM transformation_executions
             WHERE source_clip_id = ?1
             ORDER BY started_at DESC, rowid DESC
             LIMIT 25",
        )?;
        let rows = statement.query_map(params![clip_id], |row| {
            Ok(TransformationExecution {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_ref: row.get(2)?,
                target_revision: row.get(3)?,
                source_clip_id: row.get(4)?,
                trigger_kind: row.get(5)?,
                destination_kind: row.get(6)?,
                started_at: row.get(7)?,
                completed_at: row.get(8)?,
                duration_ms: row.get(9)?,
                status: row.get(10)?,
                error_summary: row.get(11)?,
            })
        })?;
        rows.collect()
    }
}
