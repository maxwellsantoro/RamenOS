# Driver Vault Template

This is a template directory for a Reference Vault. Copy this structure when creating a new driver vault.

## Directory Structure
```
vault-name/
├── datasheets/          # Hardware specifications (PDF or markdown)
├── traces/              # protocol_trace artifacts from Oracle captures
├── notes.md             # Quirks, init sequences, dependencies
└── README.md            # This file
```

## Usage
1. Create a new directory under `drivers/reference_vaults/` named after the hardware.
2. Populate with datasheets, traces, and notes.
3. Reference this vault in Foundry gates and agent prompts.
