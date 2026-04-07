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
  inspection: ProviderInspectionLike | undefined,
  statusMessage: string | null | undefined,
  t: TFunction,
) {
  const { requiresAuth, loginAvailable } = getProviderAuthState(inspection);

  if (requiresAuth) {
    return t("agentRuntimes.status.loginRequired");
  }

  if (loginAvailable) {
    return t("agentRuntimes.status.loginAvailable");
  }

  switch (status) {
    case "ready":
      return t("agentRuntimes.status.ready");
    case "installing":
      return t("agentRuntimes.status.installing");
    case "error":
      return statusMessage || t("agentRuntimes.status.error");
    case "needs_setup":
      return t("agentRuntimes.status.setupRequired");
    default:
      return t("agentRuntimes.status.notInstalled");
  }
}
