import React, { useCallback, useEffect, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  Panel,
  BackgroundVariant,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "./sfcEditor.css";
import { StepNode } from "./StepNode";
import { ParallelNode } from "./ParallelNode";
import { PropertiesPanel } from "./PropertiesPanel";
import { SfcToolsPanel } from "./SfcToolsPanel";
import { SfcCodePanel } from "./SfcCodePanel";
import { StRuntimePanel } from "../../visual/runtime/webview/StRuntimePanel";
import { useSfc } from "./hooks/useSfc";
import {
  SfcWebviewToExtensionMessage,
  SfcExtensionToWebviewMessage,
  SfcStepNode,
  SfcTransitionEdge,
  SfcExecutionState,
} from "./types";
import { runtimeMessage } from "../../visual/runtime/runtimeMessages";
import { getVsCodeApi } from "../../visual/runtime/webview/vscodeApi";
import { useRightPaneResize } from "../../visual/runtime/webview/useRightPaneResize";
import {
  DEFAULT_RUNTIME_UI_STATE,
  type RightPaneView,
  type RuntimeUiState,
} from "../../visual/runtime/runtimeTypes";
import "../../visual/runtime/webview/rightPaneResize.css";

const vscode = getVsCodeApi();

const nodeTypes = {
  step: StepNode,
  parallelSplit: ParallelNode,
  parallelJoin: ParallelNode,
} as any; // Type assertion to avoid @xyflow/react type inference issues

/**
 * Main SFC Editor Component
 */
export const SfcEditor: React.FC = () => {
  const {
    nodes,
    edges,
    variables,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNewStep,
    addParallelSplit,
    addParallelJoin,
    updateNodeData,
    updateEdgeData,
    addActionToStep,
    updateAction,
    deleteAction,
    deleteSelected,
    autoLayout,
    importFromJson,
    exportToJson,
    updateVariables,
    highlightActiveSteps,
    updateDebugState,
    setNodes,
  } = useSfc();

  const [selectedNode, setSelectedNode] = useState<SfcStepNode | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<SfcTransitionEdge | null>(null);
  const [executionState, setExecutionState] = useState<SfcExecutionState | null>(null);
  const [runtimeState, setRuntimeState] = useState<RuntimeUiState>(
    DEFAULT_RUNTIME_UI_STATE
  );
  const [rightPaneView, setRightPaneView] = useState<RightPaneView>("io");
  const [showCodePanel, setShowCodePanel] = useState(false);
  const [generatedCode, setGeneratedCode] = useState<string | null>(null);
  const [codeErrors, setCodeErrors] = useState<string[]>([]);

  const {
    rightPaneStyle,
    resizeHandleClassName,
    resizeHandleProps,
  } = useRightPaneResize("sfc");

  // Handle messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent<SfcExtensionToWebviewMessage>) => {
      const message = event.data;

      switch (message.type) {
        case "init":
        case "update":
          try {
            if (message.content) {
              const workspace = JSON.parse(message.content);
              importFromJson(workspace);
            }
          } catch (error) {
            console.error("Failed to parse SFC workspace:", error);
            vscode.postMessage({
              type: "error",
              error: String(error),
            } as SfcWebviewToExtensionMessage);
          }
          break;

        case "executionState":
          setExecutionState(message.state);
          // Update active steps highlighting
          highlightActiveSteps(message.state.activeSteps || []);
          // Update debug state if present
          if (message.state.breakpoints !== undefined) {
            updateDebugState(
              message.state.breakpoints,
              message.state.currentStep || null,
              handleToggleBreakpoint
            );
          }
          break;

        case "executionStopped":
          setExecutionState(null);
          // Clear active step indicators
          highlightActiveSteps([]);
          break;

        case "runtime.state":
          setRuntimeState(message.state);
          if (!message.state.isExecuting) {
            setExecutionState(null);
            highlightActiveSteps([]);
          }
          break;

        case "runtime.error":
          console.error("SFC runtime error:", message.message);
          break;

        case "validationResult":
          // Handle validation errors
          console.log("Validation result:", message.errors);
          if (message.errors.length === 0) {
            console.log("SFC validation passed");
          }
          break;

        case "codeGenerated":
          // Handle code generation result
          if (message.code) {
            setGeneratedCode(message.code);
            setCodeErrors(message.errors || []);
            setShowCodePanel(true);
          }
          break;
      }
    };

    window.addEventListener("message", handleMessage);

    // Notify extension that webview is ready
    vscode.postMessage({ type: "ready" } as SfcWebviewToExtensionMessage);

    return () => window.removeEventListener("message", handleMessage);
  }, [importFromJson, highlightActiveSteps, updateDebugState]);

  // Save changes to document
  const handleSave = useCallback(() => {
    const workspace = exportToJson();
    const content = JSON.stringify(workspace, null, 2);
    vscode.postMessage({
      type: "save",
      content,
    } as SfcWebviewToExtensionMessage);
  }, [exportToJson]);

  const handleOpenRuntimePanel = useCallback(() => {
    vscode.postMessage(runtimeMessage.openPanel() as SfcWebviewToExtensionMessage);
  }, []);

  // Handle selection changes
  const handleNodeClick = useCallback(
    (_event: React.MouseEvent, node: SfcStepNode) => {
      setSelectedNode(node);
      setSelectedEdge(null);
    },
    []
  );

  const handleEdgeClick = useCallback(
    (_event: React.MouseEvent, edge: SfcTransitionEdge) => {
      setSelectedEdge(edge);
      setSelectedNode(null);
    },
    []
  );

  const handlePaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, []);

  // Toolbar actions
  const handleAddStep = useCallback(() => {
    addNewStep();
  }, [addNewStep]);

  const handleAddParallelSplit = useCallback(() => {
    addParallelSplit();
  }, [addParallelSplit]);

  const handleAddParallelJoin = useCallback(() => {
    addParallelJoin();
  }, [addParallelJoin]);

  const handleDelete = useCallback(() => {
    deleteSelected();
    setSelectedNode(null);
    setSelectedEdge(null);
  }, [deleteSelected]);

  const handleValidate = useCallback(() => {
    vscode.postMessage({
      type: "validate",
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleGenerateST = useCallback(() => {
    vscode.postMessage({
      type: "generateST",
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleAutoLayout = useCallback(() => {
    autoLayout();
  }, [autoLayout]);

  const handleToggleCodePanel = useCallback(() => {
    setShowCodePanel((prev) => !prev);
  }, []);

  const handleCopyCode = useCallback(() => {
    if (generatedCode) {
      navigator.clipboard.writeText(generatedCode);
    }
  }, [generatedCode]);

  // Debug control handlers
  const handleDebugPause = useCallback(() => {
    vscode.postMessage({
      type: "debugPause",
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleDebugResume = useCallback(() => {
    vscode.postMessage({
      type: "debugResume",
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleDebugStepOver = useCallback(() => {
    vscode.postMessage({
      type: "debugStepOver",
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleToggleBreakpoint = useCallback((stepId: string) => {
    vscode.postMessage({
      type: "toggleBreakpoint",
      stepId,
    } as SfcWebviewToExtensionMessage);
  }, []);

  const handleCloseProperties = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, []);

  const hasSelection = Boolean(selectedNode || selectedEdge);

  return (
    <div className="sfc-editor" style={{ width: "100%", height: "100vh", display: "flex" }}>
      {/* Main editor area */}
      <div style={{ flex: 1, minWidth: 0, position: "relative" }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodeClick={handleNodeClick}
          onEdgeClick={handleEdgeClick}
          onPaneClick={handlePaneClick}
          nodeTypes={nodeTypes}
          fitView
          snapToGrid
          snapGrid={[15, 15]}
          defaultEdgeOptions={{
            type: "smoothstep",
            animated: true,
            markerEnd: {
              type: "arrowclosed" as any,
              width: 20,
              height: 20,
            },
            style: {
              stroke: "var(--vscode-editorWidget-border)",
              strokeWidth: 2,
            },
          }}
          style={{
            background: "var(--vscode-editor-background)",
          }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={20}
            size={1}
            color="var(--vscode-editorWidget-border)"
          />
          <Controls />
          <MiniMap
            nodeColor={(node) => {
              const data = node.data as any;
              if (data?.isActive) {
                return "#4caf50";
              }
              if (data?.isCurrentDebugStep) {
                return "#FFA500";
              }
              if (data?.type === "initial") {
                return "#2196f3";
              }
              if (data?.nodeType === "parallelSplit" || data?.nodeType === "parallelJoin") {
                return "#9c27b0";
              }
              return "#757575";
            }}
            style={{
              backgroundColor: "var(--vscode-editor-background)",
              border: "1px solid var(--vscode-panel-border)",
            }}
          />

          {/* Info Panel */}
          <Panel
            position="bottom-right"
            style={{
              padding: "8px 12px",
              backgroundColor: "var(--vscode-editor-background)",
              border: "1px solid var(--vscode-panel-border)",
              borderRadius: "4px",
              fontSize: "12px",
            }}
          >
            <div>Steps: {nodes.length}</div>
            <div>Transitions: {edges.length}</div>
            {selectedNode && <div>Selected: {selectedNode.data.label}</div>}
            {executionState && executionState.activeSteps.length > 0 && (
              <div style={{ color: "#4caf50", fontWeight: 600 }}>
                Active: {executionState.activeSteps.length}
              </div>
            )}
          </Panel>
        </ReactFlow>

        {showCodePanel && (
          <SfcCodePanel
            code={generatedCode}
            errors={codeErrors}
            onCopy={handleCopyCode}
          />
        )}
      </div>

      <div className={resizeHandleClassName} {...resizeHandleProps} />

        {/* Properties Panel (Sidebar) */}
        <div
          style={{
            ...rightPaneStyle,
            borderLeft: "1px solid var(--vscode-panel-border)",
            backgroundColor: "var(--vscode-sideBar-background)",
            display: "flex",
            flexDirection: "column",
            overflowY: "auto",
            overflowX: "hidden",
          }}
          className="right-pane-resizable"
        >
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
              gap: "6px",
              padding: "8px",
              borderBottom: "1px solid var(--vscode-panel-border)",
              position: "sticky",
              top: 0,
              zIndex: 3,
              background: "var(--vscode-sideBar-background)",
            }}
          >
            {(["io", "settings", "tools"] as RightPaneView[]).map((view) => (
              <button
                key={view}
                type="button"
                onClick={() => setRightPaneView(view)}
                style={{
                  border: "1px solid var(--vscode-button-border)",
                  borderRadius: "4px",
                  background:
                    rightPaneView === view
                      ? "var(--vscode-button-background)"
                      : "var(--vscode-button-secondaryBackground)",
                  color:
                    rightPaneView === view
                      ? "var(--vscode-button-foreground)"
                      : "var(--vscode-button-secondaryForeground)",
                  borderColor:
                    rightPaneView === view
                      ? "var(--vscode-focusBorder)"
                      : "var(--vscode-button-border)",
                  padding: "5px 8px",
                  fontSize: "11px",
                  fontWeight: 600,
                  cursor: "pointer",
                }}
                aria-pressed={rightPaneView === view}
              >
                {view === "io" ? "I/O" : view === "settings" ? "Settings" : "Tools"}
              </button>
            ))}
          </div>

          {rightPaneView === "tools" ? (
            <>
              <SfcToolsPanel
                onAddStep={handleAddStep}
                onAddParallelSplit={handleAddParallelSplit}
                onAddParallelJoin={handleAddParallelJoin}
                onDelete={handleDelete}
                onValidate={handleValidate}
                onGenerateST={handleGenerateST}
                onAutoLayout={handleAutoLayout}
                onSave={handleSave}
                onToggleCodePanel={handleToggleCodePanel}
                showCodePanel={showCodePanel}
                hasSelection={hasSelection}
              />
              {(selectedNode || selectedEdge) && (
                <PropertiesPanel
                  selectedNode={selectedNode}
                  selectedEdge={selectedEdge}
                  variables={variables}
                  onUpdateNode={updateNodeData}
                  onUpdateEdge={updateEdgeData}
                  onAddAction={addActionToStep}
                  onUpdateAction={updateAction}
                  onDeleteAction={deleteAction}
                  onUpdateVariables={updateVariables}
                  onClose={handleCloseProperties}
                />
              )}
              {!selectedNode && !selectedEdge && (
                <div
                  style={{
                    padding: "16px",
                    fontSize: "12px",
                    color: "var(--vscode-descriptionForeground)",
                    textAlign: "center",
                  }}
                >
                  <div style={{ marginBottom: "8px", fontSize: "24px" }}>📋</div>
                  <div>Select a step or transition to view properties</div>
                </div>
              )}
            </>
          ) : (
            <StRuntimePanel
              activeView={rightPaneView === "settings" ? "settings" : "io"}
              onViewChange={setRightPaneView}
              showToolsShortcut
              toolsShortcutLabel="Tools"
            />
          )}
        </div>
    </div>
  );
};
