type InvokeFn = (
  command: string,
  args: Record<string, unknown>,
) => Promise<unknown>;

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

async function invokeTool(
  invoke: InvokeFn,
  toolName: string,
  args: Record<string, unknown>,
) {
  return invoke("plugin_call_tool", {
    toolName,
    argsJson: JSON.stringify(args),
  });
}

export async function callPomodoroTool<T>(
  invoke: InvokeFn,
  toolName: string,
  args: Record<string, unknown> = {},
): Promise<T | null> {
  try {
    const result = await invokeTool(invoke, toolName, args);
    return JSON.parse(result as string) as T;
  } catch (error) {
    const message = errorMessage(error);

    if (message.includes(`Tool not found: ${toolName}`)) {
      try {
        await invoke("plugin_enable", { pluginKey: "pomodoro" });
        const result = await invokeTool(invoke, toolName, args);
        return JSON.parse(result as string) as T;
      } catch (retryError) {
        console.error(`Error calling ${toolName}:`, retryError);
        return null;
      }
    }

    console.error(`Error calling ${toolName}:`, error);
    return null;
  }
}
