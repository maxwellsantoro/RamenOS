# Reference Vaults

This directory stores the "living dossiers" for hardware drivers.
Instead of relying on tribal knowledge, we pin all context here so AI agents and human contributors have a deterministic RAG context window.

Each driver vault should contain:
- `datasheets/`: PDFs or markdown summaries of hardware specs.
- `traces/`: Known-good `protocol_trace` JSON files captured from the Oracle (e.g., Linux).
- `notes.md`: Extracted quirks, initialization sequences, and dependencies.
