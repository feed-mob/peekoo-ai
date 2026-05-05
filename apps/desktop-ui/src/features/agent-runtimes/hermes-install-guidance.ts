type RuntimePresence = {
  providerId: string;
  isInstalled: boolean;
};

export const HERMES_INSTALL_DOCS_URL =
  "https://hermes-agent.nousresearch.com/docs/getting-started/installation";

export const HERMES_INSTALL_COMMAND =
  "curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh | bash";

export const HERMES_AVAILABLE_RUNTIME_ICON_URL =
  "https://hermes-agent.nousresearch.com/docs/img/logo.png";

export function shouldShowHermesInstallGuidance(runtimes: RuntimePresence[]): boolean {
  return !runtimes.some(
    (runtime) => runtime.providerId === "hermes-agent" && runtime.isInstalled,
  );
}
