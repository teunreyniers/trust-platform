import React, { memo } from "react";
import { Handle, Position, NodeProps } from "@xyflow/react";
import type { ParallelNodeData } from "./types";

/**
 * Custom node component for SFC Parallel Split/Join markers
 * Represents IEC 61131-3 parallel divergence and convergence
 */
export const ParallelNode = memo<NodeProps<ParallelNodeData>>(({ data, selected }) => {
  const isSplit = data.nodeType === "parallelSplit";
  
  const getNodeStyle = (): React.CSSProperties => {
    return {
      padding: "4px 40px",
      borderRadius: "2px",
      border: "3px double",
      borderColor: selected
        ? "var(--vscode-focusBorder)"
        : data.isActive
        ? "var(--vscode-charts-green)"
        : "var(--vscode-editor-foreground)",
      background: data.isActive
        ? "var(--vscode-charts-green)"
        : "var(--vscode-editor-background)",
      color: data.isActive
        ? "var(--vscode-editor-background)"
        : "var(--vscode-editor-foreground)",
      fontFamily: "var(--vscode-font-family)",
      fontSize: "11px",
      fontWeight: 600,
      textAlign: "center",
      position: "relative",
      minWidth: "120px",
      height: "8px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      boxShadow: selected ? "0 0 0 2px var(--vscode-focusBorder)" : "none",
    };
  };

  return (
    <div style={getNodeStyle()}>
      {/* Top handle - for split this is input, for join this is outputs from branches */}
      <Handle
        type={isSplit ? "target" : "source"}
        position={Position.Top}
        style={{
          background: "var(--vscode-editor-foreground)",
          width: 10,
          height: 10,
          top: -5,
        }}
      />
      
      {/* Label (optional, shown on hover) */}
      <div
        style={{
          position: "absolute",
          top: "100%",
          left: "50%",
          transform: "translateX(-50%)",
          marginTop: "4px",
          fontSize: "9px",
          opacity: 0.7,
          whiteSpace: "nowrap",
        }}
        title={data.label}
      >
        {isSplit ? "⫸ SPLIT" : "⫷ JOIN"}
      </div>

      {/* Bottom handle - for split this is outputs to branches, for join this is input */}
      <Handle
        type={isSplit ? "source" : "target"}
        position={Position.Bottom}
        style={{
          background: "var(--vscode-editor-foreground)",
          width: 10,
          height: 10,
          bottom: -5,
        }}
      />
    </div>
  );
});

ParallelNode.displayName = "ParallelNode";
