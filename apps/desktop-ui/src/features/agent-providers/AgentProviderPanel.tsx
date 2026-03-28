import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Plus, AlertCircle, RefreshCw, Sparkles } from "lucide-react";
import { ProviderCard } from "./ProviderCard";
import { InstallProviderDialog } from "./InstallProviderDialog";
import { ConfigureProviderDialog } from "./ConfigureProviderDialog";
import { useAgentProviders } from "@/hooks/useAgentProviders";
import { type ProviderInfo, type InstallationMethod } from "@/types/agent-provider";

export function AgentProviderPanel() {
  const {
    installedProviders,
    availableProviders,
    defaultProvider,
    isLoading,
    installingProvider,
    error,
    refresh,
    installProvider,
    setAsDefault,
    uninstallProvider,
    getConfig,
    updateConfig,
    testConnection,
    checkPrerequisites,
    addCustomProvider,
  } = useAgentProviders();

  const [selectedProvider, setSelectedProvider] = useState<ProviderInfo | null>(null);
  const [isInstallDialogOpen, setIsInstallDialogOpen] = useState(false);
  const [isConfigureDialogOpen, setIsConfigureDialogOpen] = useState(false);

  // Load providers on mount
  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleInstall = (provider: ProviderInfo) => {
    setSelectedProvider(provider);
    setIsInstallDialogOpen(true);
  };

  const handleConfigure = (provider: ProviderInfo) => {
    setSelectedProvider(provider);
    setIsConfigureDialogOpen(true);
  };

  const handleInstallConfirm = async (
    providerId: string,
    method: InstallationMethod,
    customPath?: string
  ) => {
    await installProvider({
      providerId,
      method,
      customPath,
    });
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-text-primary">Agent Providers</h2>
          <p className="text-sm text-text-muted">Manage AI agent providers for your tasks</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={refresh} disabled={isLoading}>
            <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
          <Button
            size="sm"
            onClick={() => {
              // TODO: Add custom provider dialog
            }}
          >
            <Plus className="mr-2 h-4 w-4" />
            Add Custom
          </Button>
        </div>
      </div>

      {/* Error Alert */}
      {error && (
        <Alert className="border-red-500/50 bg-red-500/10">
          <AlertCircle className="h-4 w-4 text-red-500" />
          <AlertDescription className="text-red-200">{error}</AlertDescription>
        </Alert>
      )}

      {/* Active Provider */}
      {defaultProvider && (
        <Card className="border-primary/50 bg-primary/5">
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-base">
              <Sparkles className="h-4 w-4 text-primary" />
              Active Provider
            </CardTitle>
            <CardDescription>The provider currently used for new conversations</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-space-deep text-lg">
                {defaultProvider.isBundled ? "🔧" : "🤖"}
              </div>
              <div className="flex-1">
                <div className="font-medium text-text-primary">{defaultProvider.displayName}</div>
                <div className="text-xs text-text-muted">
                  {defaultProvider.config.defaultModel || "Default model"}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Installed Providers */}
      <div>
        <h3 className="mb-3 text-sm font-medium text-text-secondary">Installed Providers</h3>
        {installedProviders.length === 0 ? (
          <div className="rounded-lg border border-dashed border-glass-border p-8 text-center">
            <p className="text-sm text-text-muted">No providers installed yet</p>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {installedProviders.map((provider) => (
              <ProviderCard
                key={provider.providerId}
                provider={provider}
                isInstalling={installingProvider === provider.providerId}
                onSetDefault={setAsDefault}
                onInstall={handleInstall}
                onConfigure={handleConfigure}
                onUninstall={uninstallProvider}
              />
            ))}
          </div>
        )}
      </div>

      {/* Available Providers */}
      {availableProviders.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-text-secondary">Available Providers</h3>
          <div className="grid gap-4 sm:grid-cols-2">
            {availableProviders.map((provider) => (
              <ProviderCard
                key={provider.providerId}
                provider={provider}
                isInstalling={installingProvider === provider.providerId}
                onSetDefault={setAsDefault}
                onInstall={handleInstall}
                onConfigure={handleConfigure}
                onUninstall={uninstallProvider}
              />
            ))}
          </div>
        </div>
      )}

      {/* Dialogs */}
      <InstallProviderDialog
        provider={selectedProvider}
        isOpen={isInstallDialogOpen}
        onClose={() => {
          setIsInstallDialogOpen(false);
          setSelectedProvider(null);
        }}
        onInstall={handleInstallConfirm}
        onCheckPrerequisites={checkPrerequisites}
      />

      <ConfigureProviderDialog
        provider={selectedProvider}
        isOpen={isConfigureDialogOpen}
        onClose={() => {
          setIsConfigureDialogOpen(false);
          setSelectedProvider(null);
        }}
        onSave={updateConfig}
        onTest={testConnection}
      />
    </div>
  );
}
