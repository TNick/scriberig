//! Parses local plan outlines into shared plan creation requests.

/// One structured plan draft parsed from composer input.
///
/// # Public members
/// - `goal`: Plan goal from the first nonempty line.
/// - `nodes`: Ordered node drafts referenced by dependency indices.
/// - `dependencies`: One-based dependency references converted to zero-based
///   node indices.
///
/// # Private members
/// None.
pub(super) struct PlanDraftInput {
    /// Plan goal from the first nonempty line.
    pub(super) goal: String,
    /// Ordered node drafts referenced by dependency indices.
    pub(super) nodes: Vec<scriberig_app::PlanNodeDraft>,
    /// Zero-based prerequisite and dependent node indices.
    pub(super) dependencies: Vec<(usize, usize)>,
}

/// Parses a goal and optional bullet outline with explicit dependency edges.
///
/// The first nonempty line is the goal. Outline nodes use `- Title: purpose`.
/// Dependencies use `depends: 1 -> 3`, with one-based node numbers.
///
/// - `argument`: Text following the local `/plan` command.
///
/// A structured plan draft suitable for the shared application operation.
///
/// Returns an input error for malformed outline or dependency lines.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn parse_plan_argument(argument: &str) -> Result<PlanDraftInput, String> {
    let mut lines = argument
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let goal = lines
        .next()
        .filter(|line| !line.starts_with('-') && !line.starts_with("depends:"))
        .ok_or_else(|| "Start /plan with a goal on the first line.".to_owned())?
        .to_owned();
    let mut nodes = Vec::new();
    let mut dependencies = Vec::new();
    for line in lines {
        if let Some(edge) = line.strip_prefix("depends:") {
            dependencies.push(parse_dependency(edge.trim())?);
        } else if let Some(node) = line.strip_prefix("- ") {
            nodes.push(parse_node(node, nodes.len())?);
        } else {
            return Err(
                "Use '- Title: purpose' for plan nodes and 'depends: 1 -> 2' for dependencies."
                    .to_owned(),
            );
        }
    }
    if nodes.is_empty() {
        let title = goal.chars().take(80).collect::<String>();
        nodes.push(node_draft("goal".to_owned(), title, goal.clone()));
    }
    validate_dependencies(&dependencies, nodes.len())?;
    Ok(PlanDraftInput {
        goal,
        nodes,
        dependencies,
    })
}

/// Parses one outline line into a stable ordered node draft.
///
/// - `value`: Text following the outline marker.
/// - `index`: Zero-based position used to create a stable request key.
///
/// A node draft with bounded title and full purpose.
///
/// Returns an input error when title or purpose is empty.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_node(value: &str, index: usize) -> Result<scriberig_app::PlanNodeDraft, String> {
    let (title, purpose) = value
        .split_once(':')
        .ok_or_else(|| "Plan nodes use '- Title: purpose'.".to_owned())?;
    let title = title.trim();
    let purpose = purpose.trim();
    if title.is_empty() || purpose.is_empty() {
        return Err("Plan node title and purpose must not be empty.".to_owned());
    }
    Ok(node_draft(
        format!("node-{}", index + 1),
        title.chars().take(80).collect(),
        purpose.to_owned(),
    ))
}

/// Creates one node draft with the shared default result contract.
///
/// - `key`: Stable request-local node key.
/// - `title`: Bounded tree label.
/// - `purpose`: Worker instruction for the node.
///
/// A node draft attached beneath the plan anchor.
/// # Returns
/// The operation result.
fn node_draft(key: String, title: String, purpose: String) -> scriberig_app::PlanNodeDraft {
    scriberig_app::PlanNodeDraft {
        key,
        parent_index: None,
        title,
        purpose,
        expected_result: Some(scriberig_session::ExpectedResult {
            kind: "summary".to_owned(),
            requires_handle: false,
        }),
        semantic_scope: Vec::new(),
        assignment: None,
    }
}

/// Parses one one-based prerequisite to dependent node reference.
///
/// - `value`: Dependency edge in `N -> M` form.
///
/// Zero-based prerequisite and dependent node indices.
///
/// Returns an input error for malformed or zero indices.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_dependency(value: &str) -> Result<(usize, usize), String> {
    let (prerequisite, dependent) = value
        .split_once("->")
        .ok_or_else(|| "Dependencies use 'depends: 1 -> 2'.".to_owned())?;
    let prerequisite = parse_node_number(prerequisite.trim())?;
    let dependent = parse_node_number(dependent.trim())?;
    Ok((prerequisite - 1, dependent - 1))
}

/// Parses one positive one-based plan node number.
///
/// - `value`: Node number text.
///
/// Positive node number.
///
/// Returns an input error when the value is not a positive integer.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_node_number(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "Dependency node numbers start at 1.".to_owned())
}

/// Ensures every dependency endpoint names an existing distinct plan node.
///
/// - `dependencies`: Parsed zero-based edges.
/// - `node_count`: Number of nodes in the draft.
///
/// Unit when all references are in range and non-reflexive.
///
/// Returns an input error when an endpoint is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn validate_dependencies(dependencies: &[(usize, usize)], node_count: usize) -> Result<(), String> {
    if dependencies.iter().any(|(prerequisite, dependent)| {
        *prerequisite >= node_count || *dependent >= node_count || prerequisite == dependent
    }) {
        return Err("Dependency edges must reference two different existing nodes.".to_owned());
    }
    Ok(())
}
