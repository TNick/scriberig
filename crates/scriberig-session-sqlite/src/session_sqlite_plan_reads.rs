//! Implements semantic plan reads for the SQLite session repository.

macro_rules! shared_session_plan_read_methods {
    () => {
        /// Loads one compact read projection without reconstructing plan history.
        ///
        /// # Parameters
        /// - `session`: Stable identity of the requested session.
        ///
        /// # Returns
        /// The compact projection, or `None` when the session is absent.
        ///
        /// # Errors
        /// Returns a persistence diagnostic when the session cannot be decoded.
        fn load_read_projection(
            &self,
            session: scriberig_core::EntityId,
        ) -> Result<Option<scriberig_session::Session>, String> {
            self.load_session_read_projection(session)
        }

        /// Loads one exact plan revision from the plan root's snapshot history.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan_id`: Stable plan identity.
        /// - `snapshot`: Exact universal snapshot to resolve.
        ///
        /// # Returns
        /// The plan projection, or `None` when outside root history.
        ///
        /// # Errors
        /// Returns a backend diagnostic when semantic history cannot be read.
        fn load_plan_at_snapshot(
            &self,
            session: scriberig_core::EntityId,
            plan_id: scriberig_core::EntityId,
            snapshot: scriberig_core::SnapshotId,
        ) -> Result<Option<scriberig_session::Plan>, String> {
            let root = crate::semantic_plans::plan_root(session, plan_id)?;
            let Some(current) = self.semantic_store.read_root(&root) else {
                return Ok(None);
            };
            let ancestors = self
                .semantic_store
                .ancestors(current)
                .map_err(|error| format!("plan history read failed: {error:?}"))?;
            if snapshot != current && !ancestors.contains(&snapshot) {
                return Ok(None);
            }
            SqliteSessionStore::load_plan_at_snapshot(self, session, plan_id, snapshot).map(Some)
        }

        /// Loads a current or exact-snapshot plan projection.
        ///
        /// Current compatibility projections are decoded through the full
        /// semantic snapshot loader until compact SQLite assembly is available.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan_id`: Stable plan identity.
        /// - `snapshot`: Exact snapshot, or `None` for the current root.
        ///
        /// # Returns
        /// The plan projection, or `None` when the plan/snapshot is absent.
        ///
        /// # Errors
        /// Returns a backend diagnostic when semantic state cannot be read.
        fn load_plan_projection(
            &self,
            session: scriberig_core::EntityId,
            plan_id: scriberig_core::EntityId,
            snapshot: Option<scriberig_core::SnapshotId>,
        ) -> Result<Option<scriberig_session::Plan>, String> {
            self.load_plan_projection(session, plan_id, snapshot)
        }

        /// Loads derived rollups matching one exact current plan snapshot.
        ///
        /// A missing or stale cache is rebuilt from canonical plan state.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan_id`: Stable plan identity.
        /// - `snapshot`: Exact canonical plan snapshot.
        ///
        /// # Returns
        /// Snapshot-valid node rollups, or `None` for a historical snapshot.
        ///
        /// # Errors
        /// Returns a diagnostic when cache recovery or reading fails.
        fn load_plan_rollups(
            &self,
            session: scriberig_core::EntityId,
            plan_id: scriberig_core::EntityId,
            snapshot: scriberig_core::SnapshotId,
        ) -> Result<
            Option<
                std::collections::BTreeMap<
                    scriberig_core::EntityId,
                    scriberig_session::PlanNodeRollup,
                >,
            >,
            String,
        > {
            let root = crate::semantic_plans::plan_root(session, plan_id)?;
            if self.semantic_store.read_root(&root) != Some(snapshot) {
                return Ok(None);
            }
            if let Some(rollups) = self.read_plan_rollup_index(session, plan_id, snapshot)? {
                return Ok(Some(rollups));
            }
            self.rebuild_plan_rollup_index(session, plan_id)?;
            self.read_plan_rollup_index(session, plan_id, snapshot)
        }

        /// Loads branch results while skipping unrelated session history.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        ///
        /// # Returns
        /// Stored branch results, or `None` when the session is absent.
        ///
        /// # Errors
        /// Returns a diagnostic when the session payload cannot be decoded.
        fn load_plan_branch_results(
            &self,
            session: scriberig_core::EntityId,
        ) -> Result<Option<Vec<scriberig_session::BranchResult>>, String> {
            let connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            let payload = rusqlite::OptionalExtension::optional(connection.query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                rusqlite::params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            ))
            .map_err(|error| error.to_string())?;
            payload
                .as_deref()
                .map(crate::payload::decode_branch_results)
                .transpose()
        }
    };
}
