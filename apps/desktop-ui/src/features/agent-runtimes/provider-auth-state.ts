import type { RuntimeInspectionResult, RuntimeStatus } from "@/types/agent-runtime";
import type { TFunction } from "i18next";

type ProviderInspectionLike = Pick<RuntimeInspectionResult, "authRequired" | "authMethods"> | null | undefined;

export function getProviderAuthState(inspection: ProviderInspectionLike) {
  const requiresAuth = inspection?.authRequired === true;
  const loginAvailable = !requiresAuth && (inspection?.authMethods.length ?? 0) > 0;

  return {
    requiresAuth,
    loginAvailable,
  };
}

export function getProviderStatusText(
  status: RuntimeStatus,
  inspection?: ProviderInspectionLike,
  statusMessage?: string | null,
  t?: TFunction,
) {
  const { requiresAuth, loginAvailable } = getProviderAuthState(inspection);

  if (requiresAuth) {
    return t ? t("agentRuntimes.status.loginRequired") : "Login Required";
  }

  if (loginAvailable) {
    return t ? t("agentRuntimes.status.loginAvailable") : "Login Available";
  }

  switch (status) {
    case "ready":
      return t ? t("agentRuntimes.status.ready") : "Ready";
    case "installing":
      return t ? t("agentRuntimes.status.installing") : "Installing...";
    case "error":
      return statusMessage || (t ? t("agentRuntimes.status.error") : "Error");
    case "needs_setup":
      return t ? t("agentRuntimes.status.setupRequired") : "Setup Required";
    default:
      return t ? t("agentRuntimes.status.notInstalled") : "Not Installed";
  }
}
