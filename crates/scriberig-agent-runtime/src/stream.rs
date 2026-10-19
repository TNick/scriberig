//! Assembles OpenAI-compatible SSE fragments into normalized model responses.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::provider::{
    FinishReason, ModelResponse, ProviderError, ProviderMetadata, ToolCall, Usage,
};

/// One provider-neutral stream fragment used by the assembler.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StreamChunk {
    /// Incremental assistant text.
    pub text: Option<String>,
    /// Tool-call identifier by provider stream index.
    pub tool_id: Option<(usize, String)>,
    /// Tool name by provider stream index.
    pub tool_name: Option<(usize, String)>,
    /// Incremental JSON argument text by provider stream index.
    pub tool_arguments: Option<(usize, String)>,
    /// Terminal finish reason when supplied.
    pub finish_reason: Option<FinishReason>,
    /// Incremental usage metadata.
    pub usage: Option<Usage>,
    /// Whether the provider sent its terminal stream marker.
    pub done: bool,
}

/// Assembles fragmented text and structured tool arguments safely.
#[derive(Default)]
pub struct StreamAssembler {
    text: String,
    tool_calls: BTreeMap<usize, PendingToolCall>,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
    done: bool,
}

/// Accumulates one fragmented provider tool call.
#[derive(Default)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl StreamAssembler {
    /// Creates an empty response assembler.
    ///
    /// A new assembler with no response fragments.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the accumulated assistant-text length.
    ///
    /// The number of UTF-8 bytes accumulated so far.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn text_len(&self) -> usize {
        self.text.len()
    }

    /// Returns text appended after a previous byte offset.
    ///
    /// - `offset`: Previous value returned by [`Self::text_len`].
    ///
    /// The newly appended text, or no text when the offset is unchanged.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn text_since(&self, offset: usize) -> Option<&str> {
        self.text.get(offset..).filter(|text| !text.is_empty())
    }

    /// Adds one normalized fragment without parsing incomplete JSON.
    ///
    /// - `self`: Stream assembler receiving the fragment.
    /// - `chunk`: Normalized provider fragment.
    ///
    /// Unit after the fragment is accepted.
    ///
    /// Returns an error when a completed stream is given more data.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn push(&mut self, chunk: StreamChunk) -> Result<(), ProviderError> {
        if self.done {
            return Err(ProviderError::MalformedResponse(
                "stream emitted data after completion".to_owned(),
            ));
        }
        if let Some(text) = chunk.text {
            self.text.push_str(&text);
        }
        let index = chunk
            .tool_id
            .as_ref()
            .map(|value| value.0)
            .or_else(|| chunk.tool_name.as_ref().map(|value| value.0))
            .or_else(|| chunk.tool_arguments.as_ref().map(|value| value.0));
        if let Some(index) = index {
            let call = self.tool_calls.entry(index).or_default();
            if let Some((_, id)) = chunk.tool_id {
                call.id.push_str(&id);
            }
            if let Some((_, name)) = chunk.tool_name {
                call.name.push_str(&name);
            }
            if let Some((_, arguments)) = chunk.tool_arguments {
                call.arguments.push_str(&arguments);
            }
        }
        if chunk.finish_reason.is_some() {
            self.finish_reason = chunk.finish_reason;
        }
        if chunk.usage.is_some() {
            self.usage = chunk.usage;
        }
        self.done = chunk.done;
        Ok(())
    }

    /// Parses one OpenAI-compatible SSE data line.
    ///
    /// - `self`: Stream assembler receiving the line.
    /// - `line`: One complete `data:` line or terminal marker.
    ///
    /// `true` when the line was a terminal `[DONE]` marker.
    ///
    /// Returns a structured malformed-response error for invalid JSON.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn push_sse_line(&mut self, line: &str) -> Result<bool, ProviderError> {
        let data = line.strip_prefix("data:").map_or(line.trim(), str::trim);
        if data == "[DONE]" {
            self.push(StreamChunk {
                done: true,
                ..StreamChunk::default()
            })?;
            return Ok(true);
        }
        let wire: WireChunk = serde_json::from_str(data).map_err(|_| {
            ProviderError::MalformedResponse("invalid provider stream fragment".to_owned())
        })?;
        self.push(wire.into_chunk())?;
        Ok(false)
    }

    /// Finishes and normalizes the assembled response.
    ///
    /// A complete normalized response.
    ///
    /// Returns an incomplete-stream or malformed-tool-arguments error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn finish(self) -> Result<ModelResponse, ProviderError> {
        if !self.done {
            return Err(ProviderError::MalformedResponse(
                "provider stream ended before completion".to_owned(),
            ));
        }
        let tool_calls = self
            .tool_calls
            .into_values()
            .map(|call| {
                let arguments = serde_json::from_str(&call.arguments).map_err(|_| {
                    ProviderError::MalformedResponse("tool call arguments are not JSON".to_owned())
                })?;
                Ok(ToolCall {
                    id: call.id,
                    name: call.name,
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, ProviderError>>()?;
        Ok(ModelResponse {
            text: (!self.text.is_empty()).then_some(self.text),
            tool_calls,
            finish_reason: self.finish_reason.unwrap_or(FinishReason::Stop),
            usage: self.usage,
            metadata: ProviderMetadata::default(),
        })
    }
}

/// Deserializes one streamed provider chunk.
#[derive(Deserialize)]
struct WireChunk {
    choices: Vec<WireChoice>,
    usage: Option<WireUsage>,
}

/// Deserializes one streamed response choice.
#[derive(Deserialize)]
struct WireChoice {
    delta: WireDelta,
    finish_reason: Option<String>,
}

/// Deserializes one streamed delta.
#[derive(Deserialize, Default)]
struct WireDelta {
    content: Option<String>,
    tool_calls: Option<Vec<WireToolCall>>,
}

/// Deserializes one streamed tool call fragment.
#[derive(Deserialize)]
struct WireToolCall {
    index: usize,
    id: Option<String>,
    function: Option<WireFunction>,
}

/// Deserializes a streamed function fragment.
#[derive(Deserialize)]
struct WireFunction {
    name: Option<String>,
    arguments: Option<String>,
}

/// Deserializes streamed usage metadata.
#[derive(Deserialize)]
struct WireUsage {
    #[serde(rename = "prompt_tokens")]
    input: u64,
    #[serde(rename = "completion_tokens")]
    output: u64,
    #[serde(rename = "total_tokens")]
    total: u64,
}

impl WireChunk {
    /// Converts one wire chunk into a normalized stream fragment.
    ///
    /// A provider-neutral stream fragment.
    /// # Returns
    /// The operation result.
    fn into_chunk(self) -> StreamChunk {
        let choice = self.choices.into_iter().next();
        let mut chunk = StreamChunk {
            text: choice
                .as_ref()
                .and_then(|choice| choice.delta.content.clone()),
            usage: self.usage.map(|usage| Usage {
                prompt_tokens: usage.input,
                completion_tokens: usage.output,
                total_tokens: usage.total,
            }),
            ..StreamChunk::default()
        };
        if let Some(choice) = choice {
            chunk.finish_reason = choice.finish_reason.as_deref().map(map_finish_reason);
            if let Some(call) = choice.delta.tool_calls.into_iter().flatten().next() {
                chunk.tool_id = call.id.map(|id| (call.index, id));
                chunk.tool_name = call
                    .function
                    .as_ref()
                    .and_then(|function| function.name.clone())
                    .map(|name| (call.index, name));
                chunk.tool_arguments = call
                    .function
                    .and_then(|function| function.arguments)
                    .map(|arguments| (call.index, arguments));
            }
        }
        chunk
    }
}

/// Maps a compatible stream finish reason to the normalized enum.
///
/// - `reason`: Provider finish-reason spelling.
///
/// The normalized finish reason.
/// # Returns
/// The operation result.
fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "tool_calls" => FinishReason::ToolCall,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::Refusal,
        _ => FinishReason::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::StreamAssembler;

    /// Verifies private provider reasoning is excluded from normalized output.
    #[test]
    fn reasoning_content_is_not_exposed_as_assistant_text() {
        let mut assembler = StreamAssembler::new();
        assembler
            .push_sse_line(
                r#"data: {"choices":[{"delta":{"reasoning_content":"PRIVATE_CHAIN_OF_THOUGHT","content":"visible answer"},"finish_reason":"stop"}]}"#,
            )
            .expect("stream chunk");
        assembler.push_sse_line("data: [DONE]").expect("done");

        let response = assembler.finish().expect("normalized response");

        assert_eq!(response.text.as_deref(), Some("visible answer"));
        assert!(!format!("{response:?}").contains("PRIVATE_CHAIN_OF_THOUGHT"));
    }

    /// Verifies fragmented tool JSON is parsed only after completion.
    #[test]
    fn fragmented_tool_arguments_assemble_after_done() {
        let mut assembler = StreamAssembler::new();
        assembler
            .push_sse_line(
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"inspect","arguments":"{\"entity\":"}}]},"finish_reason":null}]}"#,
            )
            .expect("first fragment");
        assembler
            .push_sse_line(
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"ent_1\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            )
            .expect("second fragment");
        assert!(assembler.push_sse_line("data: [DONE]").expect("done"));

        let response = assembler.finish().expect("assembled response");

        assert_eq!(response.tool_calls[0].name, "inspect");
        assert_eq!(response.tool_calls[0].arguments["entity"], "ent_1");
    }
}
