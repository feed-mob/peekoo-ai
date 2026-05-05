const RUNTIME_ICON_OVERRIDES: Record<string, string> = {
  "hermes-agent": "https://hermes-agent.nousresearch.com/docs/img/logo.png",
};

export function getRuntimeIconUrl(runtimeId: string): string {
  return (
    RUNTIME_ICON_OVERRIDES[runtimeId] ??
    `https://cdn.agentclientprotocol.com/registry/v1/latest/${runtimeId}.svg`
  );
}
