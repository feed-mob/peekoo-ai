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
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2, AlertCircle, Check, Download } from "lucide-react";
import {
  type ProviderInfo,
  type InstallationMethod,
  type PrerequisitesCheck,
} from "@/types/agent-provider";

interface InstallProviderDialogProps {
  provider: ProviderInfo | null;
  isOpen: boolean;
  onClose: () => void;
  onInstall: (
    providerId: string,
    method: InstallationMethod,
    customPath?: string
  ) => Promise<void>;
  onCheckPrerequisites: (method: InstallationMethod) => Promise<PrerequisitesCheck>;
}

export function InstallProviderDialog({
  provider,
  isOpen,
  onClose,
  onInstall,
  onCheckPrerequisites,
}: InstallProviderDialogProps) {
  const [selectedMethod, setSelectedMethod] = useState<InstallationMethod>("npx");
  const [customPath, setCustomPath] = useState("");
  const [isChecking, setIsChecking] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);
  const [prerequisites, setPrerequisites] = useState<PrerequisitesCheck | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen && provider) {
      // Find first available method
      const availableMethod = provider.availableMethods.find((m) => m.isAvailable);
      if (availableMethod) {
        setSelectedMethod(availableMethod.id);
        checkPrerequisites(availableMethod.id);
      }
    }
  }, [isOpen, provider]);

  // Check prerequisites when method changes
  useEffect(() => {
    if (isOpen) {
      checkPrerequisites(selectedMethod);
    }
  }, [selectedMethod, isOpen]);

  const checkPrerequisites = async (method: InstallationMethod) => {
    if (!provider) return;

    setIsChecking(true);
    setError(null);
    try {
      const check = await onCheckPrerequisites(method);
      setPrerequisites(check);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsChecking(false);
    }
  };

  const handleInstall = async () => {
    if (!provider) return;

    // Validate custom path if needed
    if (selectedMethod === "custom" && !customPath.trim()) {
      setError("Please enter a path for the custom installation");
      return;
    }

    setIsInstalling(true);
    setError(null);
    try {
      await onInstall(
        provider.providerId,
        selectedMethod,
        selectedMethod === "custom" ? customPath : undefined
      );
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsInstalling(false);
    }
  };

  if (!provider) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Install {provider.displayName}</DialogTitle>
          <DialogDescription>
            Choose how you want to install this provider.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Prerequisites Alert */}
          {isChecking ? (
            <Alert className="bg-space-surface border-glass-border">
              <Loader2 className="h-4 w-4 animate-spin" />
              <AlertDescription>Checking prerequisites...</AlertDescription>
            </Alert>
          ) : prerequisites && !prerequisites.available ? (
            <Alert className="border-yellow-500/50 bg-yellow-500/10">
              <AlertCircle className="h-4 w-4 text-yellow-500" />
              <AlertDescription className="text-yellow-200">
                Missing: {prerequisites.missingComponents.join(", ")}
                {prerequisites.instructions && (
                  <div className="mt-2 text-xs">{prerequisites.instructions}</div>
                )}
              </AlertDescription>
            </Alert>
          ) : prerequisites?.available ? (
            <Alert className="border-green-500/50 bg-green-500/10">
              <Check className="h-4 w-4 text-green-500" />
              <AlertDescription className="text-green-200">
                All prerequisites satisfied
              </AlertDescription>
            </Alert>
          ) : null}

          {/* Installation Methods */}
          <RadioGroup
            value={selectedMethod}
            onValueChange={(v) => setSelectedMethod(v as InstallationMethod)}
            className="space-y-2"
          >
            {provider.availableMethods.map((method) => (
              <div
                key={method.id}
                className={`flex items-center space-x-2 rounded-lg border p-3 transition-colors ${
                  selectedMethod === method.id
                    ? "border-primary bg-primary/5"
                    : "border-glass-border bg-space-surface/50"
                } ${!method.isAvailable ? "opacity-50" : ""}`}
              >
                <RadioGroupItem
                  value={method.id}
                  id={method.id}
                  disabled={!method.isAvailable}
                />
                <Label
                  htmlFor={method.id}
                  className="flex flex-1 cursor-pointer items-center justify-between"
                >
                  <div>
                    <div className="font-medium">{method.name}</div>
                    <div className="text-xs text-text-muted">
                      {method.description}
                      {method.sizeMb && ` (~${method.sizeMb}MB)`}
                    </div>
                  </div>
                  {!method.isAvailable && (
                    <span className="text-xs text-red-400">Not Available</span>
                  )}
                </Label>
              </div>
            ))}
          </RadioGroup>

          {/* Custom Path Input */}
          {selectedMethod === "custom" && (
            <div className="space-y-2">
              <Label htmlFor="custom-path">Path to Agent Binary</Label>
              <Input
                id="custom-path"
                placeholder="/path/to/agent-binary"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                className="bg-space-deep border-glass-border"
              />
            </div>
          )}

          {/* Error Message */}
          {error && (
            <Alert className="border-red-500/50 bg-red-500/10">
              <AlertCircle className="h-4 w-4 text-red-500" />
              <AlertDescription className="text-red-200">{error}</AlertDescription>
            </Alert>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isInstalling}>
            Cancel
          </Button>
          <Button
            onClick={handleInstall}
            disabled={isInstalling || isChecking || (prerequisites && !prerequisites.available)}
          >
            {isInstalling ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Installing...
              </>
            ) : (
              <>
                <Download className="mr-2 h-4 w-4" />
                Install
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
