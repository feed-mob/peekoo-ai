import type { RuntimeInspectionResult, RuntimeStatus } from "@/types/agent-runtime";

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
) {
  const { requiresAuth, loginAvailable } = getProviderAuthState(inspection);

  if (requiresAuth) {
    return "Login Required";
  }

  if (loginAvailable) {
    return "Login Available";
  }

  switch (status) {
    case "ready":
      return "Ready";
    case "installing":
      return "Installing...";
    case "error":
      return statusMessage || "Error";
    case "needs_setup":
      return "Setup Required";
    default:
      return "Not Installed";
  }
}
