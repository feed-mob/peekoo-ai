import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ProviderAuth, ProviderCatalog } from "@/types/agent-settings";

interface AuthSectionProps {
  provider: ProviderCatalog;
  auth: ProviderAuth | undefined;
  apiKey: string;
  setApiKey: (value: string) => void;
  oauthFlowRunning: boolean;
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
  onSaveApiKey,
  onClearAuth,
  onStartOauth,
  onCheckOauth,
}: AuthSectionProps) {
  const supportsApiKey = provider.authModes.includes("api_key");
  const supportsOauth = provider.authModes.includes("oauth");

  return (
    <div className="space-y-2 rounded-md border border-glass-border p-3">
      <p className="text-xs text-text-muted">
        Auth mode: <span className="text-text-secondary">{auth?.authMode ?? "none"}</span>
      </p>

      {supportsApiKey && (
        <div className="space-y-2">
          <Input
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            type="password"
            placeholder="Enter API key"
            className="bg-space-deep border-glass-border"
          />
          <Button size="sm" onClick={() => void onSaveApiKey()}>
            Save API Key
          </Button>
        </div>
      )}

      {supportsOauth && (
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void onStartOauth()}>
            Start OAuth
          </Button>
          {oauthFlowRunning && (
            <Button size="sm" variant="secondary" onClick={() => void onCheckOauth()}>
              Check OAuth Status
            </Button>
          )}
          <p className="w-full text-xs text-text-muted">
            Clicking Start OAuth opens your browser. After you finish login and return, click
            "Check OAuth Status".
          </p>
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
