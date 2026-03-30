import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Loader2,
  AlertCircle,
  Check,
  Trash2,
  RefreshCw,
  ChevronDown,
  ChevronUp,
  Lock,
} from "lucide-react";
import {
  type RuntimeInfo,
  type RuntimeConfig,
  type RuntimeInspectionResult,
  type RuntimeAuthenticationResult,
} from "@/types/agent-runtime";
import { getProviderAuthState } from "./provider-auth-state";

interface ConfigureProviderDialogProps {
  provider: RuntimeInfo | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (providerId: string, config: RuntimeConfig) => Promise<void>;
  onInspect: (runtimeId: string) => Promise<RuntimeInspectionResult>;
  onAuthenticate: (runtimeId: string, methodId: string) => Promise<RuntimeAuthenticationResult>;
  onRefreshCapabilities: (runtimeId: string) => Promise<RuntimeInspectionResult>;
  onTest: (providerId: string) => Promise<{
    success: boolean;
    message: string;
    availableModels: string[];
    providerVersion?: string | null;
  }>;
}

export function ConfigureProviderDialog({
  provider,
  isOpen,
  onClose,
  onSave,
  onInspect,
  onAuthenticate,
  onRefreshCapabilities,
  onTest,
}: ConfigureProviderDialogProps) {
  const [config, setConfig] = useState<RuntimeConfig>({
    defaultModel: "",
    envVars: {},
    customArgs: [],
  });
  const [isLoading, setIsLoading] = useState(false);
  const [isInspecting, setIsInspecting] = useState(false);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
    providerVersion?: string | null;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [newEnvKey, setNewEnvKey] = useState("");
  const [newEnvValue, setNewEnvValue] = useState("");
  const [inspection, setInspection] = useState<RuntimeInspectionResult | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [showAuthMethods, setShowAuthMethods] = useState(false);
  const [selectedModel, setSelectedModel] = useState<string>("");

  // Load config and inspect runtime when dialog opens
  useEffect(() => {
    if (isOpen && provider) {
      setConfig({
        defaultModel: provider.config.defaultModel || "",
        envVars: { ...provider.config.envVars },
        customArgs: [...provider.config.customArgs],
      });
      setTestResult(null);
      setError(null);
      setInspection(null);
      setSelectedModel(provider.config.defaultModel || "");
      setShowAdvanced(false);
      setShowAuthMethods(false);

      if (provider.isBundled) {
        return;
      }

      // Inspect runtime to discover capabilities
      setIsInspecting(true);
      onInspect(provider.providerId)
        .then((result) => {
          setInspection(result);
          setShowAuthMethods(result.authRequired);
          // Prefer the saved default model; only fall back for the picker UI.
          const savedModel = provider.config.defaultModel || "";
          const discoveredIds = new Set(result.discoveredModels.map((model) => model.modelId));
          const modelToSelect =
            (savedModel && discoveredIds.has(savedModel) && savedModel) ||
            result.currentModelId ||
            (result.discoveredModels.length > 0 ? result.discoveredModels[0].modelId : "");
          setSelectedModel(modelToSelect);
        })
        .catch((err) => setError(String(err)))
        .finally(() => setIsInspecting(false));
    }
  }, [isOpen, onInspect, provider]);

  const handleSave = async () => {
    if (!provider) return;

    setIsLoading(true);
    setError(null);
    try {
      await onSave(provider.providerId, config);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleTest = async () => {
    if (!provider) return;

    setIsTesting(true);
    setTestResult(null);
    setError(null);
    try {
      const result = await onTest(provider.providerId);
      setTestResult(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsTesting(false);
    }
  };

  const handleRefreshCapabilities = async () => {
    if (!provider) return;

    setIsInspecting(true);
    setError(null);
    try {
      const result = await onRefreshCapabilities(provider.providerId);
      setInspection(result);
      setShowAuthMethods((current) => result.authRequired || current);
      // Refresh the picker options without overwriting the saved default model.
      const discoveredIds = new Set(result.discoveredModels.map((model) => model.modelId));
      if (!selectedModel || !discoveredIds.has(selectedModel)) {
        setSelectedModel(
          result.currentModelId ||
            (result.discoveredModels.length > 0 ? result.discoveredModels[0].modelId : "")
        );
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsInspecting(false);
    }
  };

  const handleAuthenticate = async (methodId: string) => {
    if (!provider) return;

    setIsAuthenticating(true);
    setError(null);
    setTestResult(null);
    try {
      const result = await onAuthenticate(provider.providerId, methodId);
      setTestResult({
        success: true,
        message: result.message,
      });
      if (result.status === "authenticated") {
        await handleRefreshCapabilities();
      } else {
        // Terminal login started — poll for auth completion so the user
        // doesn't have to manually click Refresh after finishing in the terminal.
        setShowAuthMethods(true);
        pollForAuthCompletion(provider.providerId);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsAuthenticating(false);
    }
  };

  const pollForAuthCompletion = (providerId: string) => {
    let attempts = 0;
    const maxAttempts = 24; // ~2 minutes at 5s intervals
    const interval = setInterval(async () => {
      attempts++;
      try {
        const result = await onRefreshCapabilities(providerId);
        if (!result.authRequired) {
          clearInterval(interval);
          setShowAuthMethods(false);
          setTestResult({ success: true, message: "Login successful." });
        }
      } catch {
        // ignore transient errors during polling
      }
      if (attempts >= maxAttempts) {
        clearInterval(interval);
      }
    }, 5000);
  };

  const handleModelChange = (modelId: string) => {
    setSelectedModel(modelId);
    setConfig((prev) => ({ ...prev, defaultModel: modelId }));
  };

  const addEnvVar = () => {
    if (newEnvKey.trim()) {
      setConfig((prev) => ({
        ...prev,
        envVars: {
          ...prev.envVars,
          [newEnvKey.trim()]: newEnvValue,
        },
      }));
      setNewEnvKey("");
      setNewEnvValue("");
    }
  };

  const removeEnvVar = (key: string) => {
    setConfig((prev) => {
      const newEnvVars = { ...prev.envVars };
      delete newEnvVars[key];
      return {
        ...prev,
        envVars: newEnvVars,
      };
    });
  };

  const addCustomArg = () => {
    setConfig((prev) => ({
      ...prev,
      customArgs: [...prev.customArgs, ""],
    }));
  };

  const updateCustomArg = (index: number, value: string) => {
    setConfig((prev) => ({
      ...prev,
      customArgs: prev.customArgs.map((arg, i) => (i === index ? value : arg)),
    }));
  };

  const removeCustomArg = (index: number) => {
    setConfig((prev) => ({
      ...prev,
      customArgs: prev.customArgs.filter((_, i) => i !== index),
    }));
  };

  if (!provider) return null;

  const hasAuthMethods = (inspection?.authMethods.length ?? 0) > 0;
  const { requiresAuth, loginAvailable } = getProviderAuthState(inspection);
  const authMethodsVisible = requiresAuth || showAuthMethods;
  const discoveredModels = inspection?.discoveredModels || [];

  return (
    <Dialog open={isOpen} onOpenChange={(open: boolean) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Configure {provider.displayName}</DialogTitle>
          <DialogDescription>
            Manage runtime settings and authentication.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Status Section */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label>Status</Label>
              <div className="flex items-center gap-2">
                {provider.status === "ready" && (
                  <span className="inline-flex items-center rounded-full bg-green-500/20 px-2 py-1 text-xs font-medium text-green-400">
                    <Check className="mr-1 h-3 w-3" />
                    Ready
                  </span>
                )}
                {provider.status === "needs_setup" && (
                  <span className="inline-flex items-center rounded-full bg-yellow-500/20 px-2 py-1 text-xs font-medium text-yellow-400">
                    <AlertCircle className="mr-1 h-3 w-3" />
                    Needs Setup
                  </span>
                )}
                {provider.status === "error" && (
                  <span className="inline-flex items-center rounded-full bg-red-500/20 px-2 py-1 text-xs font-medium text-red-400">
                    <AlertCircle className="mr-1 h-3 w-3" />
                    Error
                  </span>
                )}
              </div>
            </div>

            {isInspecting && (
              <div className="flex items-center gap-2 text-sm text-text-muted">
                <Loader2 className="h-4 w-4 animate-spin" />
                Discovering capabilities...
              </div>
            )}
          </div>

          {/* Auth Section */}
          {hasAuthMethods && (
            <div className="space-y-3 border-t border-glass-border pt-4">
              <div className="flex items-center justify-between">
                <Label className="flex items-center gap-2">
                  <Lock className="h-4 w-4" />
                  Authentication
                </Label>
                {requiresAuth && (
                  <span className="text-xs text-yellow-400">Login required</span>
                )}
                {loginAvailable && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-auto px-2 py-1 text-xs text-text-muted hover:text-text-primary"
                    onClick={() => setShowAuthMethods((current) => !current)}
                  >
                    {authMethodsVisible ? "Hide login options" : "Login available"}
                  </Button>
                )}
              </div>
              
              {authMethodsVisible ? (
                <div className="space-y-2">
                  {inspection?.authMethods.map((method) => (
                    <div key={method.id} className="flex items-center justify-between rounded-md border border-glass-border p-3">
                      <div>
                        <div className="text-sm font-medium text-text-primary">{method.name}</div>
                        {method.description && (
                          <div className="text-xs text-text-muted">{method.description}</div>
                        )}
                      </div>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleAuthenticate(method.id)}
                        disabled={isAuthenticating}
                      >
                        {isAuthenticating ? (
                          <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                        ) : null}
                        Login
                      </Button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-xs text-text-muted">
                  This runtime has optional login methods available.
                </p>
              )}
            </div>
          )}

          {/* Models Section */}
          {discoveredModels.length > 0 && !provider.isBundled && (
            <div className="space-y-3 border-t border-glass-border pt-4">
              <div className="flex items-center justify-between">
                <Label>Model</Label>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleRefreshCapabilities}
                  disabled={isInspecting}
                >
                  {isInspecting ? (
                    <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                  ) : (
                    <RefreshCw className="mr-2 h-3 w-3" />
                  )}
                  Refresh
                </Button>
              </div>
              
              <Select value={selectedModel} onValueChange={handleModelChange}>
                <SelectTrigger className="bg-space-deep border-glass-border">
                  <SelectValue placeholder="Select a model" />
                </SelectTrigger>
                <SelectContent>
                  {discoveredModels.map((model) => (
                    <SelectItem key={model.modelId} value={model.modelId}>
                      {model.name}
                      {model.description && (
                        <span className="ml-2 text-xs text-text-muted">
                          - {model.description}
                        </span>
                      )}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {provider.isBundled ? (
            <div className="space-y-3 border-t border-glass-border pt-4">
              <Label>Default Model</Label>
              <Input
                placeholder="e.g., claude-sonnet-4-6"
                value={config.defaultModel ?? ""}
                onChange={(e) =>
                  setConfig((prev) => ({ ...prev, defaultModel: e.target.value }))
                }
                className="bg-space-deep border-glass-border"
              />
              <p className="text-xs text-text-muted">
                This runtime does not use ACP model discovery. Enter a model ID manually if
                needed.
              </p>
            </div>
          ) : discoveredModels.length === 0 && !isInspecting && (
            <div className="space-y-3 border-t border-glass-border pt-4">
              <div className="flex items-center justify-between">
                <Label>Default Model</Label>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleRefreshCapabilities}
                  disabled={isInspecting}
                >
                  {isInspecting ? (
                    <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                  ) : (
                    <RefreshCw className="mr-2 h-3 w-3" />
                  )}
                  Discover Models
                </Button>
              </div>
              <Input
                placeholder="e.g., gpt-4, claude-3-5-sonnet"
                value={config.defaultModel ?? ""}
                onChange={(e) =>
                  setConfig((prev) => ({ ...prev, defaultModel: e.target.value }))
                }
                className="bg-space-deep border-glass-border"
              />
              <p className="text-xs text-text-muted">
                No models discovered. Enter a model ID manually or refresh to discover.
              </p>
            </div>
          )}

          {!provider.isBundled && (
            <div className="border-t border-glass-border pt-4">
              <Button
                variant="outline"
                onClick={handleTest}
                disabled={isTesting}
                className="w-full"
              >
                {isTesting ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Testing...
                  </>
                ) : (
                  "Test Connection"
                )}
              </Button>

              {testResult && (
                <Alert
                  className={`mt-3 ${
                    testResult.success
                      ? "border-green-500/50 bg-green-500/10"
                      : "border-red-500/50 bg-red-500/10"
                  }`}
                >
                  {testResult.success ? (
                    <Check className="h-4 w-4 text-green-500" />
                  ) : (
                    <AlertCircle className="h-4 w-4 text-red-500" />
                  )}
                  <AlertDescription
                    className={testResult.success ? "text-green-200" : "text-red-200"}
                  >
                    {testResult.message}
                    {testResult.providerVersion && (
                      <div className="mt-1 text-xs">Version: {testResult.providerVersion}</div>
                    )}
                  </AlertDescription>
                </Alert>
              )}
            </div>
          )}

          {/* Advanced Section (Collapsible) */}
          <Collapsible open={showAdvanced} onOpenChange={setShowAdvanced}>
            <CollapsibleTrigger asChild>
              <Button variant="ghost" className="w-full justify-between">
                <span>Advanced Settings</span>
                {showAdvanced ? (
                  <ChevronUp className="h-4 w-4" />
                ) : (
                  <ChevronDown className="h-4 w-4" />
                )}
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="space-y-4 pt-2">
              {/* Environment Variables */}
              <div className="space-y-3">
                <Label>Environment Variables</Label>

                {/* Existing env vars */}
                {Object.entries(config.envVars).map(([key, value]) => (
                  <div key={key} className="flex items-center gap-2">
                    <Input
                      value={key}
                      disabled
                      className="flex-1 bg-space-surface/50 border-glass-border"
                    />
                    <Input
                      type="password"
                      value={value}
                      disabled
                      className="flex-1 bg-space-surface/50 border-glass-border"
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => removeEnvVar(key)}
                      className="text-red-400 hover:bg-red-500/10"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}

                {/* Add new env var */}
                <div className="flex items-center gap-2">
                  <Input
                    placeholder="Variable name"
                    value={newEnvKey}
                    onChange={(e) => setNewEnvKey(e.target.value)}
                    className="flex-1 bg-space-deep border-glass-border"
                  />
                  <Input
                    type="password"
                    placeholder="Value"
                    value={newEnvValue}
                    onChange={(e) => setNewEnvValue(e.target.value)}
                    className="flex-1 bg-space-deep border-glass-border"
                  />
                  <Button size="sm" variant="outline" onClick={addEnvVar}>
                    Add
                  </Button>
                </div>
              </div>

              {/* Custom Arguments */}
              <div className="space-y-3">
                <Label>Custom Arguments</Label>
                {config.customArgs.map((arg, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <Input
                      placeholder="--flag or value"
                      value={arg}
                      onChange={(e) => updateCustomArg(index, e.target.value)}
                      className="flex-1 bg-space-deep border-glass-border"
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => removeCustomArg(index)}
                      className="text-red-400 hover:bg-red-500/10"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
                <Button size="sm" variant="outline" onClick={addCustomArg}>
                  Add Argument
                </Button>
              </div>
            </CollapsibleContent>
          </Collapsible>

          {/* Error */}
          {error && (
            <Alert className="border-red-500/50 bg-red-500/10">
              <AlertCircle className="h-4 w-4 text-red-500" />
              <AlertDescription className="text-red-200">{error}</AlertDescription>
            </Alert>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isLoading}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : (
              "Save Changes"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
