import { useCallback, useState } from "react";
import {
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  Connection,
  NodeChange,
  EdgeChange,
  MarkerType,
} from "@xyflow/react";
import type {
  SfcStepNode,
  SfcTransitionEdge,
  SfcWorkspace,
  StepType,
  SfcAction,
} from "../types";

const INITIAL_NODES: SfcStepNode[] = [
  {
    id: "step_init",
    type: "default",
    position: { x: 250, y: 50 },
    data: {
      label: "Init",
      type: "initial" as StepType,
      actions: [],
    },
  },
];

const INITIAL_EDGES: SfcTransitionEdge[] = [];

/**
 * Custom hook for managing SFC state and operations
 */
export function useSfc() {
  const [nodes, setNodes] = useState<SfcStepNode[]>(INITIAL_NODES);
  const [edges, setEdges] = useState<SfcTransitionEdge[]>(INITIAL_EDGES);
  const [variables, setVariables] = useState<SfcWorkspace["variables"]>([]);

  /**
   * Handle node changes (drag, select, etc.)
   */
  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setNodes((nds) => applyNodeChanges(changes, nds) as SfcStepNode[]);
  }, []);

  /**
   * Handle edge changes
   */
  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    setEdges((eds) => applyEdgeChanges(changes, eds) as SfcTransitionEdge[]);
  }, []);

  /**
   * Handle new connection (transition) between steps
   */
  const onConnect = useCallback((connection: Connection) => {
    const newEdge: SfcTransitionEdge = {
      ...connection,
      id: `trans_${Date.now()}`,
      type: "default",
      markerEnd: {
        type: MarkerType.ArrowClosed,
      },
      data: {
        condition: "TRUE",
        label: "TRUE",
      },
    };

    setEdges((eds) => addEdge(newEdge, eds) as SfcTransitionEdge[]);
  }, []);

  /**
   * Add a new step to the diagram
   */
  const addNewStep = useCallback((type: StepType = "normal") => {
    const stepCount = nodes.length;
    const newStep: SfcStepNode = {
      id: `step_${Date.now()}`,
      type: "default",
      position: { x: 250, y: 150 + stepCount * 100 },
      data: {
        label: `Step${stepCount}`,
        type,
        actions: [],
      },
    };

    setNodes((nds) => [...nds, newStep]);
    return newStep.id;
  }, [nodes.length]);

  /**
   * Add a parallel split node
   */
  const addParallelSplit = useCallback(() => {
    const nodeCount = nodes.length;
    const newNode = {
      id: `split_${Date.now()}`,
      type: "parallelSplit",
      position: { x: 250, y: 150 + nodeCount * 100 },
      data: {
        label: "Parallel Split",
        nodeType: "parallelSplit" as const,
        branchCount: 2,
      },
    };

    setNodes((nds) => [...nds, newNode as any]);
    return newNode.id;
  }, [nodes.length]);

  /**
   * Add a parallel join node
   */
  const addParallelJoin = useCallback(() => {
    const nodeCount = nodes.length;
    const newNode = {
      id: `join_${Date.now()}`,
      type: "parallelJoin",
      position: { x: 250, y: 150 + nodeCount * 100 },
      data: {
        label: "Parallel Join",
        nodeType: "parallelJoin" as const,
        branchCount: 2,
      },
    };

    setNodes((nds) => [...nds, newNode as any]);
    return newNode.id;
  }, [nodes.length]);

  /**
   * Update node data
   */
  const updateNodeData = useCallback(
    (nodeId: string, updates: Partial<SfcStepNode["data"]>) => {
      setNodes((nds) =>
        nds.map((node) =>
          node.id === nodeId
            ? { ...node, data: { ...node.data, ...updates } }
            : node
        )
      );
    },
    []
  );

  /**
   * Update edge (transition) data
   */
  const updateEdgeData = useCallback(
    (edgeId: string, updates: Partial<SfcTransitionEdge["data"]>) => {
      setEdges((eds) =>
        eds.map((edge) =>
          edge.id === edgeId
            ? {
                ...edge,
                data: { ...edge.data, ...updates },
                label: updates.label || edge.label,
              }
            : edge
        )
      );
    },
    []
  );

  /**
   * Add action to a step
   */
  const addActionToStep = useCallback(
    (stepId: string, action: SfcAction) => {
      setNodes((nds) =>
        nds.map((node) => {
          if (node.id === stepId) {
            const currentActions = node.data.actions || [];
            return {
              ...node,
              data: {
                ...node.data,
                actions: [...currentActions, action],
              },
            };
          }
          return node;
        })
      );
    },
    []
  );

  /**
   * Update action in a step
   */
  const updateAction = useCallback(
    (stepId: string, actionId: string, updates: Partial<SfcAction>) => {
      setNodes((nds) =>
        nds.map((node) => {
          if (node.id === stepId) {
            const updatedActions =
              node.data.actions?.map((action) =>
                action.id === actionId ? { ...action, ...updates } : action
              ) || [];
            return {
              ...node,
              data: {
                ...node.data,
                actions: updatedActions,
              },
            };
          }
          return node;
        })
      );
    },
    []
  );

  /**
   * Delete action from a step
   */
  const deleteAction = useCallback((stepId: string, actionId: string) => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === stepId) {
          const filteredActions =
            node.data.actions?.filter((action) => action.id !== actionId) || [];
          return {
            ...node,
            data: {
              ...node.data,
              actions: filteredActions,
            },
          };
        }
        return node;
      })
    );
  }, []);

  /**
   * Delete selected nodes and edges
   */
  const deleteSelected = useCallback(() => {
    setNodes((nds) => nds.filter((node) => !node.selected));
    setEdges((eds) => eds.filter((edge) => !edge.selected));
  }, []);

  /**
   * Auto layout - arrange nodes vertically
   */
  const autoLayout = useCallback(() => {
    setNodes((nds) => {
      const sorted = [...nds].sort((a, b) => {
        // Initial steps first
        if (a.data.type === "initial" && b.data.type !== "initial") return -1;
        if (a.data.type !== "initial" && b.data.type === "initial") return 1;
        return a.data.label.localeCompare(b.data.label);
      });

      return sorted.map((node, index) => ({
        ...node,
        position: { x: 250, y: 50 + index * 150 },
      }));
    });
  }, []);

  /**
   * Import SFC workspace from JSON
   */
  const importFromJson = useCallback((workspace: SfcWorkspace) => {
    // Convert steps to nodes
    const stepNodes: SfcStepNode[] = workspace.steps.map((step) => ({
      id: step.id,
      type: "default",
      position: { x: step.x || 250, y: step.y || 50 },
      data: {
        label: step.name,
        type: (step.initial ? "initial" : "normal") as StepType,
        actions: step.actions || [],
      },
    }));

    // Convert parallel splits to nodes
    const splitNodes = (workspace.parallelSplits || []).map((split) => ({
      id: split.id,
      type: "parallelSplit",
      position: { x: split.x || 250, y: split.y || 150 },
      data: {
        label: split.name,
        nodeType: "parallelSplit" as const,
        branchCount: split.branchIds.length,
      },
    }));

    // Convert parallel joins to nodes
    const joinNodes = (workspace.parallelJoins || []).map((join) => ({
      id: join.id,
      type: "parallelJoin",
      position: { x: join.x || 250, y: join.y || 300 },
      data: {
        label: join.name,
        nodeType: "parallelJoin" as const,
        branchCount: join.branchIds.length,
      },
    }));

    // Combine all nodes
    const allNodes = [...stepNodes, ...splitNodes, ...joinNodes];

    // Convert transitions to edges
    const newEdges: SfcTransitionEdge[] = workspace.transitions.map((trans) => ({
      id: trans.id,
      source: trans.sourceStepId,
      target: trans.targetStepId,
      type: "default",
      markerEnd: {
        type: MarkerType.ArrowClosed,
      },
      data: {
        condition: trans.condition,
        label: trans.name || trans.condition,
        priority: trans.priority,
      },
      label: trans.name || trans.condition,
    }));

    setNodes(allNodes as any);
    setEdges(newEdges);
    setVariables(workspace.variables || []);
  }, []);

  /**
   * Export SFC to workspace JSON
   */
  const exportToJson = useCallback((): SfcWorkspace => {
    // Separate steps from parallel nodes
    const stepNodes = nodes.filter(node => 
      node.type === "default" || !node.type
    );
    
    const parallelSplitNodes = nodes.filter(node => 
      node.type === "parallelSplit"
    );
    
    const parallelJoinNodes = nodes.filter(node => 
      node.type === "parallelJoin"
    );

    const steps = stepNodes.map((node) => ({
      id: node.id,
      name: node.data.label,
      initial: node.data.type === "initial",
      x: node.position.x,
      y: node.position.y,
      actions: node.data.actions || [],
    }));

    const transitions = edges.map((edge, index) => ({
      id: edge.id,
      name: (edge.data?.label || `T${index + 1}`) as string,
      condition: edge.data?.condition || "TRUE",
      sourceStepId: edge.source,
      targetStepId: edge.target,
      priority: edge.data?.priority,
    }));

    // Build parallel splits
    const parallelSplits = parallelSplitNodes.map((node) => {
      // Find all outgoing edges to get branch IDs
      const branchIds = edges
        .filter(e => e.source === node.id)
        .map(e => e.target);
      
      return {
        id: node.id,
        name: node.data.label,
        x: node.position.x,
        y: node.position.y,
        branchIds,
      };
    });

    // Build parallel joins
    const parallelJoins = parallelJoinNodes.map((node) => {
      // Find all incoming edges to get branch IDs
      const branchIds = edges
        .filter(e => e.target === node.id)
        .map(e => e.source);
      
      // Find outgoing edge to get next step
      const outgoingEdge = edges.find(e => e.source === node.id);
      
      return {
        id: node.id,
        name: node.data.label,
        x: node.position.x,
        y: node.position.y,
        branchIds,
        nextStepId: outgoingEdge?.target,
      };
    });

    return {
      name: "SFC_Program",
      steps,
      transitions,
      parallelSplits: parallelSplits.length > 0 ? parallelSplits : undefined,
      parallelJoins: parallelJoins.length > 0 ? parallelJoins : undefined,
      variables: variables || [],
      metadata: {
        modified: new Date().toISOString(),
        version: "1.0",
      },
    };
  }, [nodes, edges, variables]);

  /**
   * Update variables
   */
  const updateVariables = useCallback(
    (newVariables: SfcWorkspace["variables"]) => {
      setVariables(newVariables);
    },
    []
  );

  /**
   * Highlight active steps during execution
   */
  const highlightActiveSteps = useCallback((activeStepIds: string[]) => {
    setNodes((nds) =>
      nds.map((node) => ({
        ...node,
        data: {
          ...node.data,
          isActive: activeStepIds.includes(node.id),
        },
      }))
    );
  }, []);

  /**
   * Update debug state (breakpoints and current debug step)
   */
  const updateDebugState = useCallback(
    (
      breakpoints: string[],
      currentDebugStep: string | null,
      onToggleBreakpoint: (stepId: string) => void
    ) => {
      setNodes((nds) =>
        nds.map((node) => ({
          ...node,
          data: {
            ...node.data,
            hasBreakpoint: breakpoints.includes(node.id),
            isCurrentDebugStep: currentDebugStep === node.id,
            onToggleBreakpoint: () => onToggleBreakpoint(node.id),
          },
        }))
      );
    },
    []
  );

  return {
    nodes,
    edges,
    variables,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNewStep,
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
    addParallelSplit,
    addParallelJoin,
    setNodes,
    setEdges,
  };
}
