import { useState, useRef, useEffect, useCallback } from "react";
import { motion } from "framer-motion";
import { MessageSquarePlus, Send, Settings2, Lock, RefreshCw } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { QuickProviderSwitcher } from "@/features/agent-runtimes/QuickProviderSwitcher";
import { ConfigureProviderDialog } from "@/features/agent-runtimes/ConfigureProviderDialog";
import { useAgentProviders } from "@/hooks/useAgentProviders";
import { ChatMessage } from "./ChatMessage";
import { ChatSettingsPanel } from "./settings/ChatSettingsPanel";
import { useChatSession } from "./chat-session";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function ChatPanel() {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const [showSettings, setShowSettings] = useState(false);
  const [currentModelDisplay, setCurrentModelDisplay] = useState<string | null>(null);
  const [showLoginDialog, setShowLoginDialog] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const { messages, isTyping, authRequired, clearAuthRequired, sendMessage, startNewChat } = useChatSession();
  const {
    providers,
    defaultProvider,
    refresh,
    setAsDefault,
    getRuntimeDefaults,
    inspectRuntime,
    authenticateRuntime,
    launchNativeRuntimeLogin,
    refreshRuntimeCapabilities,
    updateConfig,
    testConnection,
  } = useAgentProviders();

  const scrollToBottom = useCallback(() => {
    requestAnimationFrame(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  useEffect(() => {
    const win = getCurrentWindow();
    const unlistenPromise = win.listen("tauri://focus", () => {
      scrollToBottom();
    });
    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, [scrollToBottom]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let cancelled = false;

    if (!defaultProvider) {
      setCurrentModelDisplay(null);
      return;
    }

    // Show the configured model immediately — no async gap.
    if (defaultProvider.config.defaultModel) {
      setCurrentModelDisplay(defaultProvider.config.defaultModel);
      return;
    }

    // No configured model — try to discover one via inspection.
    setCurrentModelDisplay(null);
    void getRuntimeDefaults(defaultProvider.providerId)
      .then(({ model }) => {
        if (!cancelled) {
          setCurrentModelDisplay(model?.displayName ?? model?.modelId ?? null);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCurrentModelDisplay(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [defaultProvider, getRuntimeDefaults]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) {
      return;
    }

    const didSend = await sendMessage(input);
    if (didSend) {
      setInput("");
    }
  };

  const handleNewChat = async () => {
    const didStart = await startNewChat();
    if (didStart) {
      setInput("");
    }
  };

  // Find the provider that needs auth (if any)
  const authRequiredProvider = authRequired
    ? (providers.find((p) => p.providerId === authRequired.runtimeId) ?? null)
    : null;

  return (
    <div className="flex min-h-0 h-full flex-col">
      <div className="mb-3 shrink-0 space-y-3">
        <div className="flex items-center justify-between gap-2">
          <QuickProviderSwitcher
            providers={providers}
            currentProvider={defaultProvider}
            currentModelDisplay={currentModelDisplay}
            onSwitch={(providerId) => {
              void setAsDefault(providerId);
            }}
            onOpenSettings={() => setShowSettings(true)}
          />

          <div className="flex justify-end gap-2">
          <Button
            type="button"
            variant="glass"
            size="sm"
            disabled={isTyping}
            onClick={() => void handleNewChat()}
          >
            <MessageSquarePlus size={14} />
            {t("chat.newChat")}
          </Button>
          <Button
            type="button"
            variant="glass"
            size="sm"
            onClick={() => setShowSettings((prev) => !prev)}
          >
            <Settings2 size={14} />
            {showSettings ? t("chat.hideSettings") : t("chat.settings")}
          </Button>
          </div>
        </div>

        {showSettings && (
          <ChatSettingsPanel
            onClose={() => setShowSettings(false)}
          />
        )}
      </div>

      <ScrollArea className="mb-4 min-h-0 flex-1">
        {messages.length === 0 ? (
          <div className="text-center text-text-muted py-8 italic">
            {t("chat.empty")}
          </div>
        ) : (
          <div className="space-y-3 pr-4">
            {messages.map((msg) => (
              <ChatMessage key={msg.id} message={msg} />
            ))}
            {isTyping && messages.every((message) => !message.streaming) && (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-start gap-2"
              >
                <span className="text-text-muted text-sm">{t("chat.thinking")}</span>
              </motion.div>
            )}
          </div>
        )}
        <div ref={messagesEndRef} />
      </ScrollArea>

      {/* Auth required banner */}
      {authRequired && (
        <div className="mb-2 flex items-center gap-2 rounded-md border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-sm dark:border-yellow-500/40">
          <Lock className="h-4 w-4 shrink-0 text-yellow-700 dark:text-yellow-400" />
          <span className="flex-1 text-yellow-800 dark:text-yellow-200">
            {authRequiredProvider?.displayName ?? authRequired.runtimeId} {t("chat.authBanner.message")}
          </span>
          <Button
            size="sm"
            variant="outline"
            className="h-7 border-yellow-500/30 px-2 text-xs text-yellow-700 hover:bg-yellow-500/10 dark:border-yellow-500/50 dark:text-yellow-400 dark:hover:bg-yellow-500/10"
            onClick={() => setShowLoginDialog(true)}
          >
            {t("chat.authBanner.loginButton")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 px-2 text-xs text-text-muted hover:text-text-primary"
            onClick={() => clearAuthRequired()}
            title={t("common.retry")}
          >
            <RefreshCw className="h-3 w-3" />
          </Button>
        </div>
      )}

      <form onSubmit={handleSubmit} className="flex gap-2">
        <Input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={t("chat.inputPlaceholder")}
          disabled={isTyping}
          className="flex-1 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted"
        />
        <Button
          type="submit"
          disabled={isTyping || !input.trim()}
          size="icon"
          variant="gradient"
        >
          <Send size={16} />
        </Button>
      </form>

      {/* Login dialog for auth-required runtime */}
      <ConfigureProviderDialog
        provider={authRequiredProvider}
        isOpen={showLoginDialog}
        onClose={() => setShowLoginDialog(false)}
        onSave={async (providerId, config) => {
          await updateConfig(providerId, config);
        }}
        onInspect={inspectRuntime}
        onAuthenticate={authenticateRuntime}
        onNativeLogin={launchNativeRuntimeLogin}
        onRefreshCapabilities={async (runtimeId) => {
          const result = await refreshRuntimeCapabilities(runtimeId);
          if (!result.authRequired) {
            clearAuthRequired();
          }
          return result;
        }}
        onTest={async (providerId) => {
          return testConnection(providerId);
        }}
      />
    </div>
  );
}
