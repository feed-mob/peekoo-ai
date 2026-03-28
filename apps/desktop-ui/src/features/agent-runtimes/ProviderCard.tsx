import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  type RuntimeInfo,
  type RuntimeLlmProviderInfo,
  type RuntimeModelInfo,
  type RuntimeStatus,
} from "@/types/agent-runtime";
import { Star, Download, Settings, Trash2, Check, AlertCircle, Loader2 } from "lucide-react";

interface ProviderCardProps {
  provider: RuntimeInfo;
  isInstalling: boolean;
  getRuntimeDefaults: (
    runtimeId: string
  ) => Promise<{ provider: RuntimeLlmProviderInfo | null; model: RuntimeModelInfo | null }>;
  onSetDefault: (providerId: string) => void;
  onInstall: (provider: RuntimeInfo) => void;
  onConfigure: (provider: RuntimeInfo) => void;
  onUninstall: (providerId: string) => void;
}

function getStatusIcon(status: RuntimeStatus) {
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

function getStatusText(status: RuntimeStatus, statusMessage?: string | null) {
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

export function ProviderCard({
  provider,
  isInstalling,
  getRuntimeDefaults,
  onSetDefault,
  onInstall,
  onConfigure,
  onUninstall,
}: ProviderCardProps) {
  const statusIcon = getStatusIcon(provider.status);
  const statusText = getStatusText(provider.status, provider.statusMessage);
  const [runtimeProvider, setRuntimeProvider] = useState<RuntimeLlmProviderInfo | null>(null);
  const [runtimeModel, setRuntimeModel] = useState<RuntimeModelInfo | null>(null);

  useEffect(() => {
    let cancelled = false;

    if (!provider.isInstalled) {
      setRuntimeProvider(null);
      setRuntimeModel(null);
      return;
    }

    void getRuntimeDefaults(provider.providerId)
      .then(({ provider: llmProvider, model }) => {
        if (!cancelled) {
          setRuntimeProvider(llmProvider);
          setRuntimeModel(model);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setRuntimeProvider(null);
          setRuntimeModel(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [getRuntimeDefaults, provider.isInstalled, provider.providerId]);

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

      {/* Status */}
      <div className="mb-4 flex items-center gap-2 text-xs">
        {statusIcon}
        <span className={provider.status === "error" ? "text-red-400" : "text-text-secondary"}>
          {statusText}
        </span>
      </div>

      {/* Actions */}
      <div className="flex gap-2">
        {provider.isInstalled ? (
          <>
            {!provider.isDefault && provider.status === "ready" && (
              <Button
                size="sm"
                variant="outline"
                className="flex-1"
                onClick={() => onSetDefault(provider.providerId)}
              >
                Set Default
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

      {/* Runtime Defaults */}
      {provider.isInstalled && (runtimeProvider || runtimeModel || provider.config.defaultModel) && (
        <div className="mt-3 space-y-1 border-t border-glass-border pt-3 text-xs text-text-muted">
          {runtimeProvider && (
            <div>
              LLM Provider: <span className="text-text-secondary">{runtimeProvider.displayName ?? runtimeProvider.providerId}</span>
            </div>
          )}
          {(runtimeModel || provider.config.defaultModel) && (
            <div>
              Model: <span className="text-text-secondary">{runtimeModel?.displayName ?? runtimeModel?.modelId ?? provider.config.defaultModel}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
