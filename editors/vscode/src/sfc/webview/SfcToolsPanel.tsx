import React from "react";

interface SfcToolsPanelProps {
  onAddStep: () => void;
  onAddParallelSplit?: () => void;
  onAddParallelJoin?: () => void;
  onDelete: () => void;
  onValidate: () => void;
  onGenerateST: () => void;
  onAutoLayout: () => void;
  onSave: () => void;
  onToggleCodePanel?: () => void;
  showCodePanel?: boolean;
  hasSelection: boolean;
}

const buttonStyle: React.CSSProperties = {
  padding: "6px 8px",
  border: "1px solid var(--vscode-button-border)",
  borderRadius: "4px",
  background: "var(--vscode-button-secondaryBackground)",
  color: "var(--vscode-button-secondaryForeground)",
  fontSize: "12px",
  cursor: "pointer",
};

/**
 * SFC Tools Panel - appears in Tools tab
 */
export const SfcToolsPanel: React.FC<SfcToolsPanelProps> = ({
  onAddStep,
  onAddParallelSplit,
  onAddParallelJoin,
  onDelete,
  onValidate,
  onGenerateST,
  onAutoLayout,
  onSave,
  onToggleCodePanel,
  showCodePanel = false,
  hasSelection,
}) => {
  return (
    <section
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "8px",
        padding: "10px 8px",
        borderBottom: "1px solid var(--vscode-panel-border)",
      }}
      aria-label="SFC tools"
    >
      <div style={{ fontSize: "12px", fontWeight: 600 }}>SFC Tools</div>
      
      {/* Add Elements */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
        <button type="button" style={buttonStyle} onClick={onAddStep} title="Add new step">
          ➕ Add Step
        </button>
        {onAddParallelSplit && (
          <button type="button" style={buttonStyle} onClick={onAddParallelSplit} title="Add parallel split">
            ⫸ Split
          </button>
        )}
        {onAddParallelJoin && (
          <button type="button" style={buttonStyle} onClick={onAddParallelJoin} title="Add parallel join">
            ⫷ Join
          </button>
        )}
        <button type="button" style={buttonStyle} onClick={onAutoLayout} title="Auto arrange steps">
          📐 Layout
        </button>
      </div>

      {/* Actions */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
        <button type="button" style={buttonStyle} onClick={onValidate} title="Validate SFC">
          ✓ Validate
        </button>
        <button type="button" style={buttonStyle} onClick={onGenerateST} title="Generate ST code">
          📄 Generate
        </button>
        {onToggleCodePanel && (
          <button
            type="button"
            style={{
              ...buttonStyle,
              background: showCodePanel
                ? "var(--vscode-button-background)"
                : "var(--vscode-button-secondaryBackground)",
              color: showCodePanel
                ? "var(--vscode-button-foreground)"
                : "var(--vscode-button-secondaryForeground)",
              borderColor: showCodePanel ? "var(--vscode-focusBorder)" : "var(--vscode-button-border)",
            }}
            onClick={onToggleCodePanel}
            title={showCodePanel ? "Hide code panel" : "Show code panel"}
          >
            {showCodePanel ? "📘" : "📗"} Code
          </button>
        )}
        <button
          type="button"
          style={{
            ...buttonStyle,
            opacity: hasSelection ? 1 : 0.55,
            cursor: hasSelection ? "pointer" : "not-allowed",
          }}
          onClick={onDelete}
          disabled={!hasSelection}
          title="Delete selected element"
        >
          🗑️ Delete
        </button>
      </div>

      {/* Save */}
      <button type="button" style={buttonStyle} onClick={onSave} title="Save changes">
        💾 Save
      </button>
    </section>
  );
};
