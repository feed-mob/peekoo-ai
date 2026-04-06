import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Download, Check, ExternalLink } from "lucide-react";
import type { RegistryAgent } from "@/types/agent-registry";

interface RegistryAgentCardProps {
  agent: RegistryAgent;
  onInstall: () => void;
  isInstalling?: boolean;
}

export function RegistryAgentCard({ agent, onInstall, isInstalling = false }: RegistryAgentCardProps) {
  const { t } = useTranslation();
  // Get icon URL or use a default
  const iconUrl = agent.iconUrl || `https://cdn.agentclientprotocol.com/registry/v1/latest/${agent.registryId}.svg`;

  return (
    <Card className={agent.isInstalled ? "border-green-500/30" : undefined}>
      <CardHeader className="pb-3">
        <div className="flex items-start gap-3">
          {/* Icon */}
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-white dark:bg-white/10 overflow-hidden p-1">
            <img
              src={iconUrl}
              alt={agent.name}
              className="h-8 w-8 object-contain"
              onError={(e) => {
                // Fallback to emoji if icon fails to load
                (e.target as HTMLImageElement).style.display = "none";
                (e.target as HTMLImageElement).parentElement!.textContent = "🤖";
              }}
            />
          </div>

          <div className="flex-1 min-w-0">
            <CardTitle className="text-sm font-medium text-text-primary truncate">
              {agent.name}
            </CardTitle>
            <CardDescription className="text-xs text-text-muted">
              v{agent.version}
            </CardDescription>
          </div>

          {/* Install Status */}
          {agent.isInstalled ? (
            <Badge variant="default" className="shrink-0 bg-green-500/15 text-green-700 border-green-500/30 dark:bg-green-500/20 dark:text-green-400 dark:border-green-500/30">
              <Check className="mr-1 h-3 w-3" />
              {t("agentRuntimes.installed")}
            </Badge>
          ) : !agent.isSupportedOnCurrentPlatform ? (
            <Badge variant="secondary" className="shrink-0">
              {t("agentRuntimes.unsupported")}
            </Badge>
          ) : null}
        </div>
      </CardHeader>

      <CardContent className="pt-0">
        <p className="text-xs text-text-muted line-clamp-2 mb-3">
          {agent.description}
        </p>

        {/* Supported Methods */}
        <div className="flex flex-wrap gap-1 mb-3">
          {agent.supportedMethods.map((method) => (
            <Badge key={method} variant="outline" className="text-xs">
              {method.toUpperCase()}
            </Badge>
          ))}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs text-text-muted">
            {agent.authors.length > 0 && (
              <span className="truncate max-w-[120px]">
                by {agent.authors[0]}
              </span>
            )}
            {agent.website && (
              <a
                href={agent.website}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-text-primary transition-colors"
                onClick={(e) => e.stopPropagation()}
              >
                <ExternalLink className="h-3 w-3" />
              </a>
            )}
          </div>

          {!agent.isInstalled && (
            <Button
              size="sm"
              variant={agent.isSupportedOnCurrentPlatform ? "default" : "outline"}
              onClick={onInstall}
              disabled={!agent.isSupportedOnCurrentPlatform || isInstalling}
            >
              {agent.isSupportedOnCurrentPlatform ? (
                <>
                  <Download className="mr-1 h-3 w-3" />
                  {isInstalling ? t("agentRuntimes.installing") : t("agentRuntimes.install")}
                </>
              ) : (
                t("agentRuntimes.unsupported")
              )}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
