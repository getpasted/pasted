use super::DbState;
use rusqlite::{params, OptionalExtension, Result};

impl DbState {
    pub fn get_content_type_groups(
        &self,
        include_archived: bool,
    ) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, label, sort_order, is_builtin, is_archived
             FROM content_type_groups WHERE ?1 OR is_archived = 0
             ORDER BY is_archived, sort_order, label COLLATE NOCASE",
        )?;
        let groups: Result<Vec<_>> = statement
            .query_map(params![include_archived], |row| {
                Ok(crate::content_types::ContentTypeGroupDefinition {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    sort_order: row.get(2)?,
                    is_builtin: row.get(3)?,
                    is_archived: row.get(4)?,
                    defaults: None,
                })
            })?
            .collect();
        groups.map(|mut groups: Vec<_>| {
            for group in &mut groups {
                if group.is_builtin {
                    group.defaults = crate::content_types::content_type_group_defaults(&group.id);
                }
            }
            groups
        })
    }

    pub fn create_content_type_group(
        &self,
        input: &crate::content_types::ContentTypeGroupInput,
    ) -> Result<crate::content_types::ContentTypeGroupDefinition> {
        crate::content_types::validate_content_type_group_input(input)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO content_type_groups (id, label, sort_order, is_builtin, is_archived) VALUES (?1, ?2, ?3, 0, 0)",
            params![input.id, input.label.trim(), input.sort_order],
        )?;
        drop(conn);
        let created = self
            .get_content_type_groups(true)?
            .into_iter()
            .find(|item| item.id == input.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_group_created",
            &format!("Created content type group \"{}\"", created.label),
        );
        Ok(created)
    }

    pub fn update_content_type_group(
        &self,
        id: &str,
        input: &crate::content_types::ContentTypeGroupInput,
    ) -> Result<crate::content_types::ContentTypeGroupDefinition> {
        if id != input.id {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group IDs cannot be changed".into(),
            ));
        }
        crate::content_types::validate_content_type_group_input(input)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_type_groups SET label = ?1, sort_order = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![input.label.trim(), input.sort_order, id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_type_groups(true)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_group_updated",
            &format!("Updated content type group \"{}\"", updated.label),
        );
        Ok(updated)
    }

    pub fn set_content_type_group_archived(&self, id: &str, archived: bool) -> Result<()> {
        let conn = self.conn.lock();
        let (is_builtin, usage_count): (bool, i64) = conn.query_row(
            "SELECT is_builtin, (SELECT COUNT(*) FROM content_types WHERE group_name = ?1) FROM content_type_groups WHERE id = ?1",
            params![id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content type groups cannot be archived".into(),
            ));
        }
        if archived && usage_count > 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Move Content Types out of this Group before archiving it".into(),
            ));
        }
        conn.execute("UPDATE content_type_groups SET is_archived = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2", params![archived, id])?;
        drop(conn);
        let _ = self.log_activity(
            if archived {
                "content_type_group_archived"
            } else {
                "content_type_group_restored"
            },
            &format!(
                "{} content type group {id}",
                if archived { "Archived" } else { "Restored" }
            ),
        );
        Ok(())
    }

    pub fn delete_content_type_group(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let (is_builtin, usage_count, label): (bool, i64, String) = conn
            .query_row(
                "SELECT is_builtin, (SELECT COUNT(*) FROM content_types WHERE group_name = ?1), label
                 FROM content_type_groups WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content type groups cannot be deleted".into(),
            ));
        }
        if usage_count > 0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Move Content Types out of this Group before deleting it".into(),
            ));
        }
        conn.execute("DELETE FROM content_type_groups WHERE id = ?1", params![id])?;
        drop(conn);
        let _ = self.log_activity(
            "content_type_group_deleted",
            &format!("Deleted content type group \"{label}\""),
        );
        Ok(())
    }

    pub fn restore_default_content_type_groups(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
            conn.execute(
                "UPDATE content_type_groups SET label = ?1, sort_order = ?2, is_archived = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?3 AND is_builtin = 1",
                params![preset.label, preset.sort_order, preset.id],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_type_groups_restored",
            "Restored built-in content type groups",
        );
        Ok(())
    }

    pub fn get_content_types(
        &self,
        include_archived: bool,
    ) -> Result<Vec<crate::content_types::ContentTypeDefinition>> {
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT types.id, types.label, types.icon, types.group_name, types.is_builtin, types.is_archived, types.conceal_clips
             FROM content_types AS types
             LEFT JOIN content_type_groups AS groups ON groups.id = types.group_name
             WHERE types.id NOT IN ('text', 'image', 'file')
               AND (?1 OR types.is_archived = 0)
             ORDER BY types.is_archived, COALESCE(groups.sort_order, 10000), types.is_builtin DESC, types.label COLLATE NOCASE",
        )?;
        let definitions: Result<Vec<_>> = statement
            .query_map(params![include_archived], |row| {
                Ok(crate::content_types::ContentTypeDefinition {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    icon: row.get(2)?,
                    group: row.get(3)?,
                    is_builtin: row.get(4)?,
                    is_archived: row.get(5)?,
                    conceal_clips: Some(row.get(6)?),
                    defaults: None,
                })
            })?
            .collect();
        definitions.map(|mut definitions: Vec<_>| {
            for definition in &mut definitions {
                if definition.is_builtin {
                    definition.defaults =
                        crate::content_types::content_type_defaults(&definition.id);
                }
            }
            definitions
        })
    }

    pub fn create_content_type(
        &self,
        input: &crate::content_types::ContentTypeInput,
    ) -> Result<crate::content_types::ContentTypeDefinition> {
        crate::content_types::validate_content_type_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let group_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
            params![input.group],
            |row| row.get(0),
        )?;
        if !group_exists {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group must exist and be active".into(),
            ));
        }
        conn.execute(
            "INSERT INTO content_types (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
             VALUES (?1, ?2, ?3, ?4, 0, 0, ?5)",
            params![input.id, input.label.trim(), input.icon, input.group, input.conceal_clips],
        )?;
        drop(conn);
        let created = self
            .get_content_types(true)?
            .into_iter()
            .find(|item| item.id == input.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_created",
            &format!("Created content type \"{}\"", created.label),
        );
        Ok(created)
    }

    pub fn update_content_type(
        &self,
        id: &str,
        input: &crate::content_types::ContentTypeInput,
    ) -> Result<crate::content_types::ContentTypeDefinition> {
        if id != input.id {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type IDs cannot be changed".into(),
            ));
        }
        crate::content_types::validate_content_type_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let group_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
            params![input.group],
            |row| row.get(0),
        )?;
        if !group_exists {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content type Group must exist and be active".into(),
            ));
        }
        let changed = conn.execute(
            "UPDATE content_types SET label = ?1, icon = ?2, group_name = ?3, conceal_clips = ?4,
                    updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
            params![
                input.label.trim(),
                input.icon,
                input.group,
                input.conceal_clips,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_types(true)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let _ = self.log_activity(
            "content_type_updated",
            &format!("Updated content type \"{}\"", updated.label),
        );
        Ok(updated)
    }

    pub fn set_content_type_archived(&self, id: &str, archived: bool) -> Result<()> {
        let conn = self.conn.lock();
        let is_builtin = conn
            .query_row(
                "SELECT is_builtin FROM content_types WHERE id = ?1",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if is_builtin {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in content types cannot be archived".into(),
            ));
        }
        let transaction = conn.unchecked_transaction()?;
        transaction.execute(
            "UPDATE content_types SET is_archived = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![archived, id],
        )?;
        if archived {
            transaction.execute(
                "UPDATE content_classifiers SET enabled = 0, updated_at = CURRENT_TIMESTAMP
                 WHERE content_type = ?1 AND is_deleted = 0",
                params![id],
            )?;
        }
        transaction.commit()?;
        drop(conn);
        let _ = self.log_activity(
            if archived {
                "content_type_archived"
            } else {
                "content_type_restored"
            },
            &format!(
                "{} content type {id}",
                if archived { "Archived" } else { "Restored" }
            ),
        );
        Ok(())
    }

    pub fn restore_default_content_types(&self) -> Result<()> {
        let conn = self.conn.lock();
        for preset in crate::content_types::CONTENT_TYPE_PRESETS {
            conn.execute(
                "UPDATE content_types SET label = ?1, icon = ?2, group_name = ?3, conceal_clips = ?4,
                        is_archived = 0, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?5 AND is_builtin = 1",
                params![preset.label, preset.icon, preset.group, preset.conceal_clips(), preset.id],
            )?;
        }
        drop(conn);
        let _ = self.log_activity(
            "content_types_restored",
            "Restored built-in content type metadata",
        );
        Ok(())
    }
}
