# Visual Editors Runtime Unification (ST Path)

Status: implementation specification for Ladder, Statechart, and Blockly runtime/debug
execution in VS Code.

## 1. Goal

All visual editors MUST execute through the same Structured Text runtime/debug pipeline as
`.st` files. Editor-specific simulation/runtime engines MUST NOT be the primary execution
path for runtime controls.

## 2. Scope

Applies to:

- `*.ladder.json`
- `*.blockly.json`
- `*.statechart.json`

Applies to runtime actions:

- mode selection (`local` / `external`)
- start / stop
- runtime I/O write / force / release

## 3. Runtime Entry Contract

For each visual source file, the extension generates:

1. Companion function-block file: sibling `*.st`
2. Launch wrapper: sibling `*.visual.runtime.st` with:
   - `PROGRAM` that instantiates the generated companion FB
   - `CONFIGURATION` + `RESOURCE` + `TASK` + `PROGRAM ... WITH TASK ...`

The wrapper file is the launch entry for local debug sessions.

## 4. Command Routing Contract

Visual editors MUST call the same extension command handlers used by ST runtime flows:

- `trust-lsp.debug.start` for local launch
- `trust-lsp.debug.attach` for external mode attach
- `trust-lsp.debug.stop` for stop
- `trust-lsp.debug.io.write` for runtime write
- `trust-lsp.debug.io.force` for runtime force
- `trust-lsp.debug.io.release` for force release

`trust-lsp.debug.start` MUST accept visual source URIs by auto-generating the companion and
wrapper before launch.

## 5. UI Behavior Contract

Visual editors share the same runtime control state model:

- `mode: local | external`
- `isExecuting: boolean`
- `status: idle | running | stopped | error`
- `lastError?: string`

Right-pane runtime controls in visual editors MUST bind to the shared command routing in
Section 4.

## 6. Current Constraints

- Attach mode currently resolves endpoint/auth from existing attach configuration flow
  (`runtime.toml`/debug attach path).
- Full end-to-end parity evidence requires manual visual verification in Extension
  Development Host for Ladder, Statechart, and Blockly editors.

## 7. Implementation Anchors

- `editors/vscode/src/visual/companionSt.ts`
- `editors/vscode/src/visual/runtime/stRuntimeCommands.ts`
- `editors/vscode/src/debug.ts`
- `editors/vscode/src/ioPanel.ts`
- `editors/vscode/src/ladder/ladderEditor.ts`
- `editors/vscode/src/statechart/stateChartEditor.ts`
- `editors/vscode/src/blockly/blocklyEditor.ts`
