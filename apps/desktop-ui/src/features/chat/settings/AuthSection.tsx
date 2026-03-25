import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ProviderAuth, ProviderCatalog } from "@/types/agent-settings";
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();
  const supportsApiKey = provider.authModes.includes("api_key");
  const supportsOauth = provider.authModes.includes("oauth");
  const isApiKeyConfigured = auth?.authMode === "api_key" && auth.configured;
  const isOauthConnected = auth?.authMode === "oauth" && auth.configured;
  const oauthPrimaryLabel = isOauthConnected ? t("chatSettings.auth.reconnectOauth") : t("chatSettings.auth.connectOauth");
  const oauthStatusLabel = isOauthConnected ? t("chatSettings.auth.connected") : t("chatSettings.auth.notConnected");

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
        {t("chatSettings.auth.mode")}: <span className="text-text-secondary">{auth?.authMode ?? t("chatSettings.auth.none")}</span>
      </p>
      {supportsOauth && (
        <p className="text-xs text-text-muted">
          {t("chatSettings.auth.oauthStatus")}: <span className="text-text-secondary">{oauthStatusLabel}</span>
        </p>
      )}

      {supportsApiKey && (
        <div className="space-y-2">
          {isApiKeyConfigured && !isEditingKey ? (
            <>
              <p className="text-xs text-emerald-300">{t("chatSettings.auth.apiKeySaved")}</p>
              <Button size="sm" variant="secondary" onClick={() => setIsEditingKey(true)}>
                {t("chatSettings.auth.updateApiKey")}
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
                placeholder={t("chatSettings.auth.enterApiKey")}
                className="bg-space-deep border-glass-border"
              />
              <div className="flex gap-2">
                <Button size="sm" onClick={() => void handleSaveApiKey()}>
                  {t("chatSettings.auth.saveApiKey")}
                </Button>
                {isEditingKey && (
                  <Button size="sm" variant="ghost" onClick={() => setIsEditingKey(false)}>
                    {t("common.cancel")}
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
              {t("chatSettings.auth.finishedLogin")}
            </Button>
          )}
          {oauthFlowRunning ? (
            <p className="w-full text-xs text-text-muted">
              {t("chatSettings.auth.browserLoginHelp")}
            </p>
          ) : isOauthConnected ? (
            <p className="w-full text-xs text-text-muted">
              {t("chatSettings.auth.connectedHelp")}
            </p>
          ) : (
            <p className="w-full text-xs text-text-muted">
              {t("chatSettings.auth.connectHelp")}
            </p>
          )}
          {oauthStatus === "failed" && oauthError ? (
            <p className="w-full text-xs text-danger">{t("chatSettings.auth.oauthFailed", { error: oauthError })}</p>
          ) : null}
          {oauthStatus === "expired" ? (
            <p className="w-full text-xs text-text-muted">{t("chatSettings.auth.oauthExpired")}</p>
          ) : null}
          {oauthStatus === "completed" && isOauthConnected ? (
            <p className="w-full text-xs text-emerald-300">{t("chatSettings.auth.oauthConnected")}</p>
          ) : null}
        </div>
      )}

      {auth?.configured && (
        <Button size="sm" variant="destructive" onClick={() => void onClearAuth()}>
          {t("chatSettings.auth.clearAuth")}
        </Button>
      )}
    </div>
  );
}
