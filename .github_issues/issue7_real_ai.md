## Description
Replace the mock AI implementation with real AI provider integration.

## Current State
- AI settings structure exists (AiSettings)
- Mock AI responses (line 342-350 in main.rs)
- AI actions defined: ExplainError, SuggestFix, GenerateCommand, SummarizeOutput
- AI can be toggled on/off

## Requirements
1. **OpenAI Integration**
   - Support GPT-3.5/GPT-4 models
   - API key management (secure storage)
   - Context-aware prompts
   - Rate limiting and error handling

2. **Anthropic Integration**
   - Support Claude models
   - Alternative to OpenAI
   - Same interface as OpenAI

3. **Local AI Support** (Optional)
   - Ollama integration
   - Run models locally
   - No API costs
   - Privacy-focused

4. **AI Features**
   - Explain command errors
   - Suggest fixes for failed commands
   - Generate commands from natural language
   - Summarize long output
   - Context from repo (if git repo)

## Implementation Details
- Add reqwest or similar for HTTP requests
- Implement API client for each provider
- Secure API key storage (OS keychain)
- Add configuration UI for API keys
- Replace mock responses with real API calls

## Files to Modify
- src/main.rs: Lines 342-350 (call_ai function)
- src/main.rs: Lines 322-339 (collect_ai_data)
- Add new module: src/ai.rs or src/ai_providers.rs
- Cargo.toml: Add dependencies (reqwest, serde_json, etc.)

## Dependencies to Add
```toml
reqwest = { version = "0.11", features = ["json"] }
tokio-util = "0.7"
keyring = "2.0"  # For secure API key storage
```

## Priority
Low - Enhancement feature, mock works for basic functionality

## Security Notes
- Never log API keys
- Store keys in OS keychain
- Allow environment variable override for CI/CD
