use super::*;

impl DbState {
    pub(super) fn init_library_items(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;
            DROP TRIGGER IF EXISTS library_items_transform_insert;
            DROP TRIGGER IF EXISTS library_items_transform_update;
            DROP TRIGGER IF EXISTS library_items_transform_delete;
            DROP TABLE IF EXISTS library_items;
            CREATE TABLE library_items (
                stable_ref TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK (kind IN ('capture', 'inspector', 'extractor', 'classifier', 'suggestion', 'operation', 'transform')),
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                group_label TEXT,
                icon TEXT NOT NULL DEFAULT 'FileText',
                enabled INTEGER CHECK (enabled IS NULL OR enabled IN (0, 1)),
                is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
                is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1)),
                sort_order INTEGER,
                revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
                input_contract TEXT NOT NULL DEFAULT 'text',
                output_contract TEXT NOT NULL DEFAULT 'preserve_type',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_library_items_kind_order
                ON library_items(kind, is_archived, sort_order, name);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('capture:clip-type-v1', 'capture', 'Clip Type',
                    'Assigns exactly one structural Text, Image, or Files type from the captured clipboard representation.',
                    'Capture', 'Shapes', NULL, 1, 0, 0, 1,
                    'clipboard_representation', 'clip_type', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('capture:source-attribution-v1', 'capture', 'Source Attribution',
                    'Records the application associated with a clipboard capture and resolves its icon when shown.',
                    'Capture', 'AppWindow', NULL, 1, 0, 10, 1,
                    'clipboard_event', 'source_attribution', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:structure-v1', 'inspector', 'Structure',
                    'Measures stable clip structure without retaining clipboard contents.',
                    'Content Analysis', 'ScanSearch', NULL, 1, 0, 0, 1,
                    'clip', 'structural_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:file-format-v1', 'inspector', 'File Format',
                    'Identifies referenced file formats from bounded byte signatures.',
                    'Content Analysis', 'FileType2', NULL, 1, 0, 10, 1,
                    'file_references', 'file_formats', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('inspector:media-metadata-v1', 'inspector', 'Media Metadata',
                    'Reads bounded audio and video metadata locally.',
                    'Content Analysis', 'FileAudio', NULL, 1, 0, 20, 1,
                    'file_references', 'media_metadata', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            VALUES ('suggestion:smart-actions-v1', 'suggestion', 'Smart Actions',
                    'Suggests saved Transforms from content-free analysis signals.',
                    'Content Analysis', 'Lightbulb', NULL, 1, 0, 0, 1,
                    'analyzable_text+structural_metadata', 'suggestions',
                    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT stable_ref, 'extractor', name, description, 'Content Analysis',
                   'ScanText', enabled, is_builtin, is_deleted, priority, 1,
                   input_contract, output_contract, created_at, updated_at
            FROM content_extractors
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                input_contract=excluded.input_contract,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled,
                 is_builtin, is_archived, sort_order, revision, input_contract,
                 output_contract, created_at, updated_at)
            SELECT classifiers.stable_ref, 'classifier', classifiers.name, classifiers.description,
                   groups.label, types.icon, classifiers.enabled, classifiers.is_builtin,
                   classifiers.is_deleted, classifiers.priority, 1, 'text',
                   'set_type:' || classifiers.content_type, classifiers.created_at, classifiers.updated_at
            FROM content_classifiers AS classifiers
            LEFT JOIN content_types AS types ON types.id = classifiers.content_type
            LEFT JOIN content_type_groups AS groups ON groups.id = types.group_name
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                group_label=excluded.group_label, icon=excluded.icon,
                enabled=excluded.enabled, is_builtin=excluded.is_builtin,
                is_archived=excluded.is_archived, sort_order=excluded.sort_order,
                output_contract=excluded.output_contract, updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'custom:' || id, 'operation', name, category, 'Wrench', enabled, 0,
                   0, row_id, 1, 'text', 'preserve_type', created_at, updated_at
            FROM custom_operations
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, group_label=excluded.group_label,
                enabled=excluded.enabled, sort_order=excluded.sort_order,
                updated_at=excluded.updated_at;

            INSERT INTO library_items
                (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract,
                 created_at, updated_at)
            SELECT 'transform:' || id, 'transform', name,
                   CASE authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,
                   'Workflow', NULL, 0,
                   0, row_id, revision, 'text', 'preserve_type', created_at, updated_at
            FROM saved_transforms
            WHERE 1 = 1
            ON CONFLICT(stable_ref) DO UPDATE SET
                name=excluded.name, sort_order=excluded.sort_order,
                revision=excluded.revision, updated_at=excluded.updated_at;

            DROP TRIGGER IF EXISTS library_items_extractor_insert;
            DROP TRIGGER IF EXISTS library_items_extractor_update;
            DROP TRIGGER IF EXISTS library_items_extractor_delete;
            DROP TRIGGER IF EXISTS library_items_classifier_insert;
            DROP TRIGGER IF EXISTS library_items_classifier_update;
            DROP TRIGGER IF EXISTS library_items_classifier_delete;
            DROP TRIGGER IF EXISTS library_items_content_type_update;
            DROP TRIGGER IF EXISTS library_items_content_group_update;
            DROP TRIGGER IF EXISTS library_items_operation_insert;
            DROP TRIGGER IF EXISTS library_items_operation_update;
            DROP TRIGGER IF EXISTS library_items_operation_delete;
            DROP TRIGGER IF EXISTS library_items_pipeline_insert;
            DROP TRIGGER IF EXISTS library_items_pipeline_update;
            DROP TRIGGER IF EXISTS library_items_pipeline_delete;

            CREATE TRIGGER library_items_extractor_insert AFTER INSERT ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_update AFTER UPDATE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              VALUES (NEW.stable_ref, 'extractor', NEW.name, NEW.description, 'Content Analysis',
                      'ScanText', NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                      1, NEW.input_contract, NEW.output_contract, NEW.created_at, NEW.updated_at);
            END;
            CREATE TRIGGER library_items_extractor_delete AFTER DELETE ON content_extractors BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_classifier_insert AFTER INSERT ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_update AFTER UPDATE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref OR stable_ref=NEW.stable_ref;
              INSERT INTO library_items
                (stable_ref, kind, name, description, group_label, icon, enabled, is_builtin,
                 is_archived, sort_order, revision, input_contract, output_contract, created_at, updated_at)
              SELECT NEW.stable_ref, 'classifier', NEW.name, NEW.description, groups.label,
                     types.icon, NEW.enabled, NEW.is_builtin, NEW.is_deleted, NEW.priority,
                     1, 'text', 'set_type:' || NEW.content_type, NEW.created_at, NEW.updated_at
              FROM content_types AS types LEFT JOIN content_type_groups AS groups ON groups.id=types.group_name
              WHERE types.id=NEW.content_type;
            END;
            CREATE TRIGGER library_items_classifier_delete AFTER DELETE ON content_classifiers BEGIN
              DELETE FROM library_items WHERE stable_ref=OLD.stable_ref;
            END;
            CREATE TRIGGER library_items_content_type_update AFTER UPDATE ON content_types BEGIN
              UPDATE library_items SET
                icon=NEW.icon,
                group_label=(SELECT label FROM content_type_groups WHERE id=NEW.group_name),
                output_contract='set_type:'||NEW.id,
                updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT stable_ref FROM content_classifiers WHERE content_type=NEW.id
              );
            END;
            CREATE TRIGGER library_items_content_group_update AFTER UPDATE ON content_type_groups BEGIN
              UPDATE library_items SET group_label=NEW.label,updated_at=CURRENT_TIMESTAMP
              WHERE kind='classifier' AND stable_ref IN (
                SELECT classifiers.stable_ref FROM content_classifiers AS classifiers
                JOIN content_types AS types ON types.id=classifiers.content_type
                WHERE types.group_name=NEW.id
              );
            END;
            CREATE TRIGGER library_items_operation_insert AFTER INSERT ON custom_operations BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('custom:'||NEW.id,'operation',NEW.name,NEW.category,'Wrench',NEW.enabled,0,0,NEW.row_id,1,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_operation_update AFTER UPDATE ON custom_operations BEGIN
              UPDATE library_items SET name=NEW.name,group_label=NEW.category,enabled=NEW.enabled,updated_at=NEW.updated_at WHERE stable_ref='custom:'||NEW.id;
            END;
            CREATE TRIGGER library_items_operation_delete AFTER DELETE ON custom_operations BEGIN
              DELETE FROM library_items WHERE stable_ref='custom:'||OLD.id;
            END;
            CREATE TRIGGER library_items_transform_insert AFTER INSERT ON saved_transforms BEGIN
              INSERT OR REPLACE INTO library_items (stable_ref,kind,name,group_label,icon,enabled,is_builtin,is_archived,sort_order,revision,input_contract,output_contract,created_at,updated_at)
              VALUES ('transform:'||NEW.id,'transform',NEW.name,CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,'Workflow',NULL,0,0,NEW.row_id,NEW.revision,'text','preserve_type',NEW.created_at,NEW.updated_at);
            END;
            CREATE TRIGGER library_items_transform_update AFTER UPDATE ON saved_transforms BEGIN
              UPDATE library_items SET name=NEW.name,group_label=CASE NEW.authoring_kind WHEN 'manual' THEN 'Local Transforms' ELSE 'Transforms' END,revision=NEW.revision,updated_at=NEW.updated_at WHERE stable_ref='transform:'||NEW.id;
            END;
            CREATE TRIGGER library_items_transform_delete AFTER DELETE ON saved_transforms BEGIN
              DELETE FROM library_items WHERE stable_ref='transform:'||OLD.id;
            END;",
        )?;
        for (index, definition) in crate::operation_registry::BUILTIN_OPERATIONS
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO library_items
                    (stable_ref, kind, name, group_label, icon, enabled, is_builtin,
                     is_archived, sort_order, revision, input_contract, output_contract)
                 VALUES (?1, 'operation', ?2, ?3, 'Wrench', 1, 1, 0, ?4, 1, 'text', 'preserve_type')
                 ON CONFLICT(stable_ref) DO UPDATE SET name=excluded.name,
                    group_label=excluded.group_label, sort_order=excluded.sort_order",
                params![
                    format!("builtin:{}", definition.key),
                    definition.name,
                    definition.category_label,
                    index as i64
                ],
            )?;
        }
        Ok(())
    }
}
