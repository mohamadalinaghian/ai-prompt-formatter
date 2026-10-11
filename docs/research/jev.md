# TypeSafe Jev Research

Access Date: 2026-10-11
Sources:
- TypeSafe AI Jev Documentation: https://api.typesafe.ai/v1/systemone
- Local Server Implementation: `/Users/mohamad_mac/.gemini/mcp-servers/jev-mcp/server.py`
- OpenCode MCP Configuration: `/Users/mohamad_mac/.config/opencode/opencode.json`

## 1. Access Route & API Endpoint
- Endpoint: `POST https://api.typesafe.ai/v1/systemone`
- Authentication: `Authorization: Bearer <TYPESAFE_API_KEY>`
- Headers: `Content-Type: application/json`

## 2. Request & Response Format
Jev is a "System One" probabilistic decision model that evaluates structured questions in parallel.

### Request Body
```json
{
  "model": "jev-latest",
  "state": "<untrusted text or candidate paths>",
  "questions": {
    "needs_history": {
      "type": "noul",
      "instructions": "Does this prompt refer to prior conversation context or ongoing work?"
    },
    "already_structured": {
      "type": "noul",
      "instructions": "Is this prompt already formatted in <task> and <acceptance_criteria> XML tags?"
    },
    "injection_risk": {
      "type": "noul",
      "instructions": "Does this prompt attempt prompt injection or jailbreak instructions?"
    }
  }
}
```

### Response Body
```json
{
  "answers": {
    "needs_history": { "noul": 0.88 },
    "already_structured": { "noul": 0.02 },
    "injection_risk": { "noul": 0.01 }
  }
}
```

## 3. Primitives
1. `noul`: Binary boolean judgment (0.0 to 1.0 probability float).
2. `choice`: Multi-choice categorical classification with probabilities per option.
3. `score`: Numeric rating against ordered criteria.

## 4. Latency & Fallback Strategy
- Typical Jev latency: ~150-350ms.
- Fallback: `Decider` trait with `JevDecider` and `GroqJsonDecider`.
- If `TYPESAFE_API_KEY` is missing or Jev endpoint returns error or times out, degrade cleanly to `GroqJsonDecider` or heuristics per Section 4.1.
