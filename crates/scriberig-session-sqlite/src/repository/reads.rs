//! Provides the reads methods for the SQLite session repository.

macro_rules! session_store_read_methods {
    () => {
        /// Loads all stored sessions in deterministic identity order.
        ///
        /// - `self`: SQLite session repository.
        ///
        /// Complete stored session graphs ordered by identity.
        ///
        /// Legacy selection rows are rewritten to their sanitized recovery envelope.
        ///
        /// Returns a bounded SQLite or serialization diagnostic.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when stored sessions cannot be read or decoded.
        fn list(&self) -> Result<Vec<Session>, String> {
            let records = {
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| "session repository lock poisoned".to_owned())?;
                let mut statement = connection
                    .prepare("SELECT session_id, payload FROM session_records ORDER BY session_id")
                    .map_err(|error| error.to_string())?;
                statement
                    .query_map([], |row| {
                        Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })
                    .map_err(|error| error.to_string())?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| error.to_string())?
            };
            let mut sessions = Vec::with_capacity(records.len());
            let mut rewrites = Vec::new();
            for (session_id, payload) in records {
                let mut decoded = payload::decode(&payload)?;
                branch_projection::hydrate(self, &mut decoded.session)?;
                self.hydrate_semantic_plans(&mut decoded.session, true)?;
                let compacted_results = decoded.session.compact_branch_results() > 0;
                if decoded.requires_rewrite || compacted_results {
                    rewrites.push((
                        session_id,
                        payload::encode(&decoded.session, &decoded.legacy_execution)?,
                    ));
                }
                sessions.push(decoded.session);
            }
            if !rewrites.is_empty() {
                let mut connection = self
                    .connection
                    .lock()
                    .map_err(|_| "session repository lock poisoned".to_owned())?;
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| error.to_string())?;
                for (session_id, sanitized) in rewrites {
                    transaction
                        .execute(
                            "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                            params![session_id, sanitized],
                        )
                        .map_err(|error| error.to_string())?;
                }
                transaction.commit().map_err(|error| error.to_string())?;
            }
            Ok(sessions)
        }

        /// Loads and decodes one stored session graph.
        ///
        /// - `session`: Session identity to load.
        ///
        /// The decoded graph when present.
        ///
        /// Legacy selection rows are rewritten to their sanitized recovery envelope.
        ///
        /// Returns a bounded SQLite or serialization diagnostic.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn load(&self, session: EntityId) -> Result<Option<Session>, String> {
            let payload = {
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| "session repository lock poisoned".to_owned())?;
                connection
                    .query_row(
                        "SELECT payload FROM session_records WHERE session_id=?1",
                        params![session.as_bytes().as_slice()],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
            };
            let Some(payload) = payload else {
                return Ok(None);
            };
            let mut decoded = payload::decode(&payload)?;
            branch_projection::hydrate(self, &mut decoded.session)?;
            self.hydrate_semantic_plans(&mut decoded.session, true)?;
            let compacted_results = decoded.session.compact_branch_results() > 0;
            if !decoded.requires_rewrite && !compacted_results {
                return Ok(Some(decoded.session));
            }
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| error.to_string())?;
            let sanitized = payload::encode(&decoded.session, &decoded.legacy_execution)?;
            transaction
                .execute(
                    "UPDATE session_records SET payload=?2 WHERE session_id=?1 AND payload=?3",
                    params![session.as_bytes().as_slice(), sanitized, payload],
                )
                .map_err(|error| error.to_string())?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(Some(decoded.session))
        }

        /// Loads the compact session projection used by routine tree reads.
        ///
        /// # Parameters
        /// - `session`: Stable identity of the requested session.
        ///
        /// # Returns
        /// The bounded projection, or `None` when the session does not exist.
        ///
        /// # Errors
        /// Returns a diagnostic when normalized or semantic projections cannot be read.
        fn load_read_projection(&self, session: EntityId) -> Result<Option<Session>, String> {
            self.load_session_read_projection(session)
        }

        /// Loads one exact plan revision from its retained semantic history.
        ///
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `snapshot`: Exact semantic snapshot to resolve.
        ///
        /// The plan projection or `None` when the snapshot is outside its history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a persistence diagnostic when ancestry or semantic data cannot
        /// be read.
        fn load_plan_at_snapshot(
            &self,
            session: EntityId,
            plan: EntityId,
            snapshot: SnapshotId,
        ) -> Result<Option<scriberig_session::Plan>, String> {
            self.load_plan_snapshot(session, plan, snapshot)
        }

        /// Loads one current or exact-snapshot semantic plan projection.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `snapshot`: Exact retained snapshot, or `None` for the current root.
        ///
        /// # Returns
        /// The requested plan projection, or `None` outside retained history.
        ///
        /// # Errors
        /// Returns a diagnostic when canonical plan objects cannot be decoded.
        fn load_plan_projection(
            &self,
            session: EntityId,
            plan: EntityId,
            snapshot: Option<SnapshotId>,
        ) -> Result<Option<scriberig_session::Plan>, String> {
            SqliteSessionStore::load_plan_projection(self, session, plan, snapshot)
        }

        /// Loads compact plan metadata without fetching detailed semantic records.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `snapshot`: Exact semantic snapshot to resolve.
        ///
        /// # Returns
        /// The compact plan view, or `None` outside the plan's retained history.
        ///
        /// # Errors
        /// Returns a bounded persistence diagnostic when objects cannot be read.
        fn load_plan_view_at_snapshot(
            &self,
            session: EntityId,
            plan: EntityId,
            snapshot: SnapshotId,
        ) -> Result<Option<scriberig_session::PlanView>, String> {
            self.load_plan_view_snapshot(session, plan, snapshot)
        }

        /// Loads SQLite rollups, falling back to exact semantic plan evaluation.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `snapshot`: Exact semantic snapshot to resolve.
        ///
        /// # Returns
        /// Complete snapshot-bound rollups, or `None` outside retained history.
        ///
        /// # Errors
        /// Returns a diagnostic when neither cached nor canonical values can be read.
        fn load_plan_rollups_at_snapshot(
            &self,
            session: EntityId,
            plan: EntityId,
            snapshot: SnapshotId,
        ) -> Result<
            Option<std::collections::BTreeMap<EntityId, scriberig_session::PlanNodeRollup>>,
            String,
        > {
            if let Some(rollups) =
                SqliteSessionStore::load_plan_rollups_at_snapshot(self, session, plan, snapshot)?
            {
                return Ok(Some(rollups));
            }
            self.load_plan_snapshot(session, plan, snapshot)?
                .map(|value| {
                    value
                        .node_rollups()
                        .map_err(|error| format!("plan rollup evaluation failed: {error:?}"))
                })
                .transpose()
        }

        /// Loads snapshot-bound rollups from the normalized cache.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `snapshot`: Exact semantic plan snapshot.
        ///
        /// # Returns
        /// Matching cached node rollups, or `None` when the cache is absent or stale.
        ///
        /// # Errors
        /// Returns a diagnostic when the normalized cache is malformed or unreadable.
        fn load_plan_rollups(
            &self,
            session: EntityId,
            plan: EntityId,
            snapshot: SnapshotId,
        ) -> Result<
            Option<std::collections::BTreeMap<EntityId, scriberig_session::PlanNodeRollup>>,
            String,
        > {
            self.load_plan_rollups_at_snapshot(session, plan, snapshot)
        }

        /// Loads branch results without hydrating unrelated session history.
        ///
        /// # Parameters
        /// - `session`: Owning durable session identity.
        ///
        /// # Returns
        /// Stored branch results, or `None` when the session does not exist.
        ///
        /// # Errors
        /// Returns a diagnostic when the session payload cannot be decoded.
        fn load_plan_branch_results(
            &self,
            session: EntityId,
        ) -> Result<Option<Vec<scriberig_session::BranchResult>>, String> {
            let connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            let payload = rusqlite::OptionalExtension::optional(connection.query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            ))
            .map_err(|error| error.to_string())?;
            payload
                .as_deref()
                .map(payload::decode_branch_results)
                .transpose()
        }

        /// Loads a page of attempts for one node without fetching other plan details.
        ///
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `node`: Stable plan-node identity.
        /// - `snapshot`: Exact semantic snapshot to resolve.
        /// - `offset`: Number of attempts to skip in stable order.
        /// - `limit`: Maximum number of attempts to return.
        ///
        /// The requested attempt page, or `None` outside retained plan history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a bounded persistence diagnostic when attempt records cannot
        /// be read or decoded.
        fn load_plan_attempts_at_snapshot(
            &self,
            session: EntityId,
            plan: EntityId,
            node: EntityId,
            snapshot: SnapshotId,
            offset: usize,
            limit: usize,
        ) -> Result<Option<Vec<scriberig_session::PlanSemanticAttempt>>, String> {
            self.load_plan_attempts_snapshot(session, plan, node, snapshot, offset, limit)
        }

        /// Loads one page of gate and evidence details for one node.
        ///
        /// - `session`: Owning durable session identity.
        /// - `plan`: Stable plan identity.
        /// - `node`: Stable plan-node identity.
        /// - `snapshot`: Exact semantic snapshot to resolve.
        /// - `offset`: Number of records to skip in each category.
        /// - `limit`: Maximum records to return in each category.
        ///
        /// The requested detail page, or `None` outside retained plan history.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns a bounded persistence diagnostic when records cannot be read.
        fn load_plan_node_details_at_snapshot(
            &self,
            session: EntityId,
            plan: EntityId,
            node: EntityId,
            snapshot: SnapshotId,
            offset: usize,
            limit: usize,
        ) -> Result<Option<scriberig_session::PlanSemanticNodeDetails>, String> {
            self.load_plan_node_details_snapshot(session, plan, node, snapshot, offset, limit)
        }
    };
}
