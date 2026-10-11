# Groq API Research

Access Date: 2026-10-11
Sources:
- Official Groq Documentation & Model Directory: https://console.groq.com/docs/models
- Groq Structured Outputs / JSON Schema: https://console.groq.com/docs/structured-outputs
- Groq Rate Limits & Pricing: https://groq.com/pricing

## 1. Supported Model IDs
- `openai/gpt-oss-120b`: Flagship open-weight model with 131,072 context window, strong reasoning and tool-use capabilities.
- `qwen/qwen3.8-27b`: 27B dense model with 131,072 context window, optimized for fast agentic workflows and code tasks.
- `openai/gpt-oss-20b`: Fast compact fallback model.

All three model IDs in the writer chain (`["openai/gpt-oss-120b", "qwen/qwen3.8-27b", "openai/gpt-oss-20b"]`) are valid and available on Groq.

## 2. Structured Outputs & JSON Schema
- Endpoint: `POST https://api.groq.com/openai/v1/chat/completions`
- Parameter: `response_format` supports:
  - `{"type": "json_schema", "json_schema": {"name": "writer_output", "strict": true, "schema": { ... }}}`
  - Fallback: `{"type": "json_object"}` with schema in system prompt.
- Temperature: 0.0 recommended for deterministic output.
- Low reasoning effort: Set `reasoning_effort: "low"` where supported by the reasoning engine.

## 3. Rate Limits & Pricing
- Developer Tier: Pay-as-you-go based on million tokens.
- Input tokens: ~$0.05 to $1.00 / M tokens depending on model size.
- Output tokens: ~$0.08 to $3.00 / M tokens.
- HTTP 429 returned on rate limit exceeding, with `retry-after` header.
- Error codes: 400 (bad request/schema mismatch), 401 (auth), 429 (rate limit), 500/503 (server error).
- Retries: Exponential backoff with jitter up to timeout.
