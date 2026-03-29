import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  type RuntimeInfo,
  type RuntimeStatus,
  type RuntimeInspectionResult,
} from "@/types/agent-runtime";
import { getProviderAuthState, getProviderStatusText } from "./provider-auth-state";
import { Star, Download, Settings, Trash2, Check, AlertCircle, Loader2, Lock, RefreshCw } from "lucide-react";

interface ProviderCardProps {
  provider: RuntimeInfo;
  isInstalling: boolean;
  onInspect: (runtimeId: string) => Promise<RuntimeInspectionResult>;
  onSetDefault: (providerId: string) => void;
  onInstall: (provider: RuntimeInfo) => void;
  onConfigure: (provider: RuntimeInfo) => void;
  onUninstall: (providerId: string) => void;
}

function getStatusIcon(status: RuntimeStatus, requiresAuth?: boolean) {
  if (requiresAuth) {
    return <Lock className="h-4 w-4 text-yellow-500" />;
  }
  switch (status) {
    case "ready":
      return <Check className="h-4 w-4 text-green-500" />;
    case "installing":
      return <Loader2 className="h-4 w-4 animate-spin text-blue-500" />;
    case "error":
      return <AlertCircle className="h-4 w-4 text-red-500" />;
    case "needs_setup":
      return <Settings className="h-4 w-4 text-yellow-500" />;
    default:
      return <Download className="h-4 w-4 text-gray-400" />;
  }
}

export function ProviderCard({
  provider,
  isInstalling,
  onInspect,
  onSetDefault,
  onInstall,
  onConfigure,
  onUninstall,
}: ProviderCardProps) {
  const [inspection, setInspection] = useState<RuntimeInspectionResult | null>(null);
  const [isInspecting, setIsInspecting] = useState(false);

  // Inspect installed runtimes to get current ACP state.
  useEffect(() => {
    let cancelled = false;

    if (!provider.isInstalled || provider.isBundled) {
      setInspection(null);
      return;
    }

    setIsInspecting(true);
    onInspect(provider.providerId)
      .then((result) => {
        if (!cancelled) {
          setInspection(result);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setInspection(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsInspecting(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [onInspect, provider.isInstalled, provider.providerId]);

  const { requiresAuth, loginAvailable } = getProviderAuthState(inspection);
  const statusIcon = getStatusIcon(provider.status, requiresAuth);
  const statusText = getProviderStatusText(provider.status, inspection, provider.statusMessage);
  const currentModel = inspection?.currentModelId || provider.config.defaultModel || "Default";

  const handleQuickRefresh = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!provider.isInstalled) return;
    
    setIsInspecting(true);
    try {
      const result = await onInspect(provider.providerId);
      setInspection(result);
    } catch {
      // Ignore errors
    } finally {
      setIsInspecting(false);
    }
  };

  const isAuthStatusActionable = requiresAuth || loginAvailable;

  return (
    <div
      className={`relative rounded-lg border p-4 transition-all ${
        provider.isDefault
          ? "border-primary bg-primary/5"
          : "border-glass-border bg-space-surface/50 hover:bg-space-surface"
      }`}
    >
      {/* Default Badge */}
      {provider.isDefault && (
        <div className="absolute -top-2 -right-2 rounded-full bg-primary p-1">
          <Star className="h-3 w-3 fill-primary-foreground text-primary-foreground" />
        </div>
      )}

      {/* Runtime Icon & Info */}
      <div className="mb-3 flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-space-deep text-lg">
            {provider.isBundled ? "🔧" : "🤖"}
          </div>
          <div>
            <h3 className="font-medium text-text-primary">{provider.displayName}</h3>
            <p className="text-xs text-text-muted">{provider.description}</p>
          </div>
        </div>
      </div>

      {/* Status & Model */}
      <div className="mb-3 space-y-2">
        <div className="flex items-center gap-2 text-xs">
          {isInspecting ? (
            <Loader2 className="h-4 w-4 animate-spin text-text-muted" />
          ) : (
            statusIcon
          )}
          {isAuthStatusActionable ? (
            <button
              type="button"
              className="text-text-secondary underline-offset-2 hover:text-text-primary hover:underline"
              onClick={() => onConfigure(provider)}
            >
              {statusText}
            </button>
          ) : (
            <span className={provider.status === "error" ? "text-red-400" : "text-text-secondary"}>
              {statusText}
            </span>
          )}
        </div>
        
        {provider.isInstalled && (
          <div className="flex items-center justify-between text-xs">
            <span className="text-text-muted">Model: </span>
            <div className="flex items-center gap-2">
              <span className="text-text-secondary truncate max-w-[150px]">{currentModel}</span>
              <Button
                size="sm"
                variant="ghost"
                className="h-5 w-5 p-0"
                onClick={handleQuickRefresh}
                disabled={isInspecting}
              >
                <RefreshCw className={`h-3 w-3 ${isInspecting ? "animate-spin" : ""}`} />
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex gap-2">
        {provider.isInstalled ? (
          <>
            {!provider.isDefault && provider.status === "ready" && !requiresAuth && (
              <Button
                size="sm"
                variant="outline"
                className="flex-1"
                onClick={() => onSetDefault(provider.providerId)}
              >
                Set Default
              </Button>
            )}
            {requiresAuth && (
              <Button
                size="sm"
                variant="outline"
                className="flex-1 border-yellow-500/50 text-yellow-400 hover:bg-yellow-500/10"
                onClick={() => onConfigure(provider)}
              >
                <Lock className="mr-1 h-3 w-3" />
                Login
              </Button>
            )}
            {loginAvailable && (
              <Button
                size="sm"
                variant="outline"
                className="flex-1"
                onClick={() => onConfigure(provider)}
              >
                <Lock className="mr-1 h-3 w-3" />
                Login
              </Button>
            )}
            <Button
              size="sm"
              variant="outline"
              className="flex-1"
              onClick={() => onConfigure(provider)}
            >
              <Settings className="mr-1 h-3 w-3" />
              Configure
            </Button>
            {!provider.isBundled && (
              <Button
                size="sm"
                variant="ghost"
                className="text-red-400 hover:bg-red-500/10 hover:text-red-500"
                onClick={() => onUninstall(provider.providerId)}
              >
                <Trash2 className="h-3 w-3" />
              </Button>
            )}
          </>
        ) : (
          <Button
            size="sm"
            className="w-full"
            disabled={isInstalling}
            onClick={() => onInstall(provider)}
          >
            {isInstalling ? (
              <>
                <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                Installing...
              </>
            ) : (
              <>
                <Download className="mr-2 h-3 w-3" />
                Install
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}
