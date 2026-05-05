import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Input } from "@/components/ui/input";
import { Plus, AlertCircle, RefreshCw, Sparkles, Search, Download } from "lucide-react";
import { ProviderCard } from "./ProviderCard";
import { RegistryAgentCard } from "./RegistryAgentCard";
import { HermesInstallGuidanceCard } from "./HermesInstallGuidanceCard";
import { InstallProviderDialog } from "./InstallProviderDialog";
import { ConfigureProviderDialog } from "./ConfigureProviderDialog";
import { AddCustomRuntimeDialog } from "./AddCustomRuntimeDialog";
import { shouldShowHermesInstallGuidance } from "./hermes-install-guidance";
import { getRuntimeIconUrl } from "./runtime-icon-url";
import { useAgentProviders } from "@/hooks/useAgentProviders";
import { useRegistryAgents } from "@/hooks/useRegistryAgents";
import { type RuntimeInfo, type InstallationMethod } from "@/types/agent-runtime";
import { useTranslation } from "react-i18next";

export function AgentProviderPanel() {
  const { t } = useTranslation();
  const {
    installedProviders,
    defaultProvider,
    isLoading,
    installingProvider,
    error,
    refresh,
    installProvider,
    setAsDefault,
    uninstallProvider,
    updateConfig,
    testConnection,
    checkPrerequisites,
    addCustomProvider,
    inspectRuntime,
    authenticateRuntime,
    launchNativeRuntimeLogin,
    refreshRuntimeCapabilities,
  } = useAgentProviders();

  // Registry agents integration
  const {
    agents: registryAgents,
    loading: registryLoading,
    error: registryError,
    hasMore,
    fetchAgents,
    searchAgents,
    loadMore,
    refresh: refreshRegistry,
    installAgent,
    installingAgentId,
  } = useRegistryAgents();

  const [searchQuery, setSearchQuery] = useState("");

  const [selectedProvider, setSelectedProvider] = useState<RuntimeInfo | null>(null);
  const [isInstallDialogOpen, setIsInstallDialogOpen] = useState(false);
  const [isConfigureDialogOpen, setIsConfigureDialogOpen] = useState(false);
  const [isAddCustomDialogOpen, setIsAddCustomDialogOpen] = useState(false);
  const showHermesGuidance = shouldShowHermesInstallGuidance(installedProviders);
  const totalAvailableCount = registryAgents.length + (showHermesGuidance ? 1 : 0);

  // Load providers on mount
  useEffect(() => {
    refresh();
    fetchAgents(true);
  }, [refresh, fetchAgents]);

  useEffect(() => {
    const timeoutId = window.setTimeout(() => {
      void searchAgents(searchQuery);
    }, 250);

    return () => window.clearTimeout(timeoutId);
  }, [searchAgents, searchQuery]);

  const handleInstall = (provider: RuntimeInfo) => {
    setSelectedProvider(provider);
    setIsInstallDialogOpen(true);
  };

  const handleConfigure = (provider: RuntimeInfo) => {
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
          <h2 className="text-lg font-semibold text-text-primary">{t("agentRuntimes.title")}</h2>
          <p className="text-sm text-text-muted">{t("agentRuntimes.description")}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={refresh} disabled={isLoading}>
            <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
            {t("agentRuntimes.refresh")}
          </Button>
          <Button
            size="sm"
            onClick={() => setIsAddCustomDialogOpen(true)}
          >
            <Plus className="mr-2 h-4 w-4" />
            {t("agentRuntimes.addRuntime")}
          </Button>
        </div>
      </div>

      {/* Error Alert */}
      {error && (
        <Alert className="border-red-500/30 bg-red-500/10 dark:border-red-500/50 dark:bg-red-500/10">
          <AlertCircle className="h-4 w-4 text-red-700 dark:text-red-500" />
          <AlertDescription className="text-red-800 dark:text-red-200">{error}</AlertDescription>
        </Alert>
      )}

      {/* Active Runtime */}
      {defaultProvider && (
        <Card className="border-primary/50 bg-primary/5">
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-base">
              <Sparkles className="h-4 w-4 text-primary" />
              {t("agentRuntimes.activeRuntime")}
            </CardTitle>
            <CardDescription>{t("agentRuntimes.activeRuntimeDescription")}</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-white dark:bg-white/10 overflow-hidden p-1">
                <img
                  src={getRuntimeIconUrl(defaultProvider.providerId)}
                  alt={defaultProvider.displayName}
                  className="h-8 w-8 object-contain"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = "none";
                    (e.target as HTMLImageElement).parentElement!.textContent = defaultProvider.isBundled ? "🔧" : "🤖";
                  }}
                />
              </div>
              <div className="flex-1">
                <div className="font-medium text-text-primary">{defaultProvider.displayName}</div>
                <div className="text-xs text-text-muted">
                  {defaultProvider.config.defaultModel || t("agentRuntimes.defaultModel")}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Installed Runtimes */}
      <div>
        <h3 className="mb-3 text-sm font-medium text-text-secondary">{t("agentRuntimes.installedRuntimes")}</h3>
        {installedProviders.length === 0 ? (
          <div className="rounded-lg border border-dashed border-glass-border p-8 text-center">
            <p className="text-sm text-text-muted">{t("agentRuntimes.noRuntimesInstalled")}</p>
          </div>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {installedProviders.map((provider) => (
              <ProviderCard
                key={provider.providerId}
                provider={provider}
                isInstalling={installingProvider === provider.providerId}
                onInspect={inspectRuntime}
                onSetDefault={setAsDefault}
                onInstall={handleInstall}
                onConfigure={handleConfigure}
                onUninstall={uninstallProvider}
              />
            ))}
          </div>
        )}
      </div>

      {/* Available Runtimes */}
      <div>
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-sm font-medium text-text-secondary">{t("agentRuntimes.availableRuntimes")}</h3>
          <span className="text-xs text-text-muted">
            {t("agentRuntimes.showingRuntimes", { count: totalAvailableCount })}
          </span>
        </div>

        {/* Search Bar */}
        <div className="mb-4 flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted" />
            <Input
              placeholder={t("agentRuntimes.searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
          <Button
            variant="outline"
            size="icon"
            onClick={() => {
              void searchAgents(searchQuery);
            }}
            disabled={registryLoading}
          >
            <Search className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            onClick={refreshRegistry}
            disabled={registryLoading}
          >
            <RefreshCw className={`h-4 w-4 ${registryLoading ? "animate-spin" : ""}`} />
          </Button>
        </div>

        {/* Registry Error */}
        {registryError && (
          <Alert className="mb-4 border-red-500/30 bg-red-500/10 dark:border-red-500/50 dark:bg-red-500/10">
            <AlertCircle className="h-4 w-4 text-red-700 dark:text-red-500" />
            <AlertDescription className="text-red-800 dark:text-red-200">
              {registryError}
            </AlertDescription>
          </Alert>
        )}

        {/* Available Runtime Grid */}
        {totalAvailableCount === 0 && !registryLoading ? (
          <div className="rounded-lg border border-dashed border-glass-border p-8 text-center">
            <p className="text-sm text-text-muted">{t("agentRuntimes.noRuntimesFound")}</p>
          </div>
        ) : (
          <>
            <div className="grid gap-4 sm:grid-cols-2">
              {showHermesGuidance && <HermesInstallGuidanceCard />}
              {registryAgents.map((agent) => (
                <RegistryAgentCard
                  key={agent.registryId}
                  agent={agent}
                  isInstalling={installingAgentId === agent.registryId}
                  onInstall={() => {
                    void installAgent(agent)
                      .then(() => refresh())
                      .catch(() => undefined);
                  }}
                />
              ))}
            </div>

            {/* Load More */}
            {hasMore && (
              <div className="mt-4 flex justify-center">
                <Button
                  variant="outline"
                  onClick={loadMore}
                  disabled={registryLoading}
                >
                  {registryLoading ? (
                    <>
                      <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                      {t("agentRuntimes.loading")}
                    </>
                  ) : (
                    <>
                      <Download className="mr-2 h-4 w-4" />
                      {t("agentRuntimes.loadMore")}
                    </>
                  )}
                </Button>
              </div>
            )}
          </>
        )}
      </div>

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
          refresh();
        }}
        onSave={updateConfig}
        onInspect={inspectRuntime}
        onAuthenticate={authenticateRuntime}
        onNativeLogin={launchNativeRuntimeLogin}
        onRefreshCapabilities={refreshRuntimeCapabilities}
        onTest={testConnection}
      />

      <AddCustomRuntimeDialog
        isOpen={isAddCustomDialogOpen}
        onClose={() => setIsAddCustomDialogOpen(false)}
        onSubmit={async ({ name, description, command, args, workingDir }) => {
          await addCustomProvider({
            name,
            description,
            command,
            args,
            workingDir,
          });
          await refresh();
        }}
      />
    </div>
  );
}
