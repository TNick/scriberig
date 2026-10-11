# Sessions

`scriberig-session` models sessions, branches, goals, decisions, tool interactions, summaries, and context-relevant session structure as reusable knowledge-domain state.

It must remain usable without an agent runtime. Agent execution consumes session APIs; session storage must not depend on model providers. Sessions use the same universal identity/revision/snapshot architecture as other domains.
