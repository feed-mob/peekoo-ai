import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ProviderAuth, ProviderCatalog } from "@/types/agent-settings";

interface AuthSectionProps {
  provider: ProviderCatalog;
  auth: ProviderAuth | undefined;
  apiKey: string;
  setApiKey: (value: string) => void;
  oauthFlowRunning: boolean;
  oauthStatus: "idle" | "pending" | "completed" | "failed" | "expired";
  oauthError: string | null;
  onSaveApiKey: () => Promise<void>;
  onClearAuth: () => Promise<void>;
  onStartOauth: () => Promise<void>;
  onCheckOauth: () => Promise<void>;
}

export function AuthSection({
  provider,
  auth,
  apiKey,
  setApiKey,
  oauthFlowRunning,
  oauthStatus,
  oauthError,
  onSaveApiKey,
  onClearAuth,
  onStartOauth,
  onCheckOauth,
}: AuthSectionProps) {
  const supportsApiKey = provider.authModes.includes("api_key");
  const supportsOauth = provider.authModes.includes("oauth");
  const isApiKeyConfigured = auth?.authMode === "api_key" && auth.configured;
  const isOauthConnected = auth?.authMode === "oauth" && auth.configured;
  const oauthPrimaryLabel = isOauthConnected ? "Reconnect OAuth" : "Connect OAuth";
  const oauthStatusLabel = isOauthConnected ? "Connected" : "Not connected";

  const [isEditingKey, setIsEditingKey] = useState(false);
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);

  const handleSaveApiKey = async () => {
    if (!apiKey.trim()) return;
    try {
      await onSaveApiKey();
      setApiKeyError(null);
      setIsEditingKey(false);
    } catch (error) {
      setApiKeyError(String(error));
    }
  };

  useEffect(() => {
    setIsEditingKey(false);
    setApiKeyError(null);
  }, [auth?.providerId]);

  return (
    <div className="space-y-2 rounded-md border border-glass-border p-3">
      <p className="text-xs text-text-muted">
        Auth mode: <span className="text-text-secondary">{auth?.authMode ?? "none"}</span>
      </p>
      {supportsOauth && (
        <p className="text-xs text-text-muted">
          OAuth status: <span className="text-text-secondary">{oauthStatusLabel}</span>
        </p>
      )}

      {supportsApiKey && (
        <div className="space-y-2">
          {isApiKeyConfigured && !isEditingKey ? (
            <>
              <p className="text-xs text-emerald-300">API key saved</p>
              <Button size="sm" variant="secondary" onClick={() => setIsEditingKey(true)}>
                Update API Key
              </Button>
            </>
          ) : (
            <>
              <Input
                value={apiKey}
                onChange={(event) => {
                  setApiKeyError(null);
                  setApiKey(event.target.value);
                }}
                type="password"
                placeholder="Enter API key"
                className="bg-space-deep border-glass-border"
              />
              <div className="flex gap-2">
                <Button size="sm" onClick={() => void handleSaveApiKey()}>
                  Save API Key
                </Button>
                {isEditingKey && (
                  <Button size="sm" variant="ghost" onClick={() => setIsEditingKey(false)}>
                    Cancel
                  </Button>
                )}
              </div>
              {apiKeyError ? <p className="text-xs text-danger">{apiKeyError}</p> : null}
            </>
          )}
        </div>
      )}

      {supportsOauth && (
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void onStartOauth()} disabled={oauthFlowRunning}>
            {oauthPrimaryLabel}
          </Button>
          {oauthFlowRunning && (
            <Button size="sm" variant="secondary" onClick={() => void onCheckOauth()}>
              I Finished Login
            </Button>
          )}
          {oauthFlowRunning ? (
            <p className="w-full text-xs text-text-muted">
              Browser login is open. Finish OAuth in the browser, then click "I Finished Login".
            </p>
          ) : isOauthConnected ? (
            <p className="w-full text-xs text-text-muted">
              OAuth is connected for this provider. Use Reconnect OAuth if you want to refresh
              credentials.
            </p>
          ) : (
            <p className="w-full text-xs text-text-muted">
              Click Connect OAuth to sign in with your provider account.
            </p>
          )}
          {oauthStatus === "failed" && oauthError ? (
            <p className="w-full text-xs text-danger">OAuth failed: {oauthError}</p>
          ) : null}
          {oauthStatus === "expired" ? (
            <p className="w-full text-xs text-text-muted">OAuth session expired. Start again.</p>
          ) : null}
          {oauthStatus === "completed" && isOauthConnected ? (
            <p className="w-full text-xs text-emerald-300">OAuth connected successfully.</p>
          ) : null}
        </div>
      )}

      {auth?.configured && (
        <Button size="sm" variant="destructive" onClick={() => void onClearAuth()}>
          Clear Auth
        </Button>
      )}
    </div>
  );
}
