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
import { Loader2, AlertCircle, Check, Trash2 } from "lucide-react";
import { type ProviderInfo, type ProviderConfig } from "@/types/agent-provider";

interface ConfigureProviderDialogProps {
  provider: ProviderInfo | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (providerId: string, config: ProviderConfig) => Promise<void>;
  onTest: (providerId: string) => Promise<{
    success: boolean;
    message: string;
    availableModels: string[];
    providerVersion?: string;
  }>;
}

export function ConfigureProviderDialog({
  provider,
  isOpen,
  onClose,
  onSave,
  onTest,
}: ConfigureProviderDialogProps) {
  const [config, setConfig] = useState<ProviderConfig>({
    defaultModel: "",
    envVars: {},
    customArgs: [],
  });
  const [isLoading, setIsLoading] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
    providerVersion?: string;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [newEnvKey, setNewEnvKey] = useState("");
  const [newEnvValue, setNewEnvValue] = useState("");

  // Load config when dialog opens
  useEffect(() => {
    if (isOpen && provider) {
      setConfig({
        defaultModel: provider.config.defaultModel || "",
        envVars: { ...provider.config.envVars },
        customArgs: [...provider.config.customArgs],
      });
      setTestResult(null);
      setError(null);
    }
  }, [isOpen, provider]);

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

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Configure {provider.displayName}</DialogTitle>
          <DialogDescription>
            Customize the settings for this provider.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Default Model */}
          <div className="space-y-2">
            <Label htmlFor="default-model">Default Model</Label>
            <Input
              id="default-model"
              placeholder="e.g., gpt-4, claude-3-5-sonnet"
              value={config.defaultModel}
              onChange={(e) =>
                setConfig((prev) => ({ ...prev, defaultModel: e.target.value }))
              }
              className="bg-space-deep border-glass-border"
            />
            <p className="text-xs text-text-muted">
              Leave empty to use the provider&apos;s default model
            </p>
          </div>

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

          {/* Test Connection */}
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
