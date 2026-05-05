import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ExternalLink } from "lucide-react";
import {
  HERMES_AVAILABLE_RUNTIME_ICON_URL,
  HERMES_INSTALL_COMMAND,
  HERMES_INSTALL_DOCS_URL,
} from "./hermes-install-guidance";

export function HermesInstallGuidanceCard() {
  const { t } = useTranslation();

  return (
    <Card className="border-dashed border-primary/40 bg-primary/5">
      <CardHeader className="pb-3">
        <div className="flex items-start gap-3">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-space-deep text-primary">
            <img
              src={HERMES_AVAILABLE_RUNTIME_ICON_URL}
              alt={t("agentRuntimes.hermesGuidance.title")}
              className="h-8 w-8 object-contain"
            />
          </div>
          <div className="min-w-0 flex-1">
            <CardTitle className="text-sm font-medium text-text-primary">
              {t("agentRuntimes.hermesGuidance.title")}
            </CardTitle>
            <CardDescription className="text-xs text-text-muted">
              {t("agentRuntimes.hermesGuidance.subtitle")}
            </CardDescription>
          </div>
          <Badge variant="outline" className="shrink-0">
            PATH
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <p className="text-xs text-text-muted">
          {t("agentRuntimes.hermesGuidance.description")}
        </p>
        <pre className="overflow-x-auto rounded-md border border-glass-border bg-space-deep p-3 text-xs text-text-secondary">
          <code>{HERMES_INSTALL_COMMAND}</code>
        </pre>
        <div className="flex items-center justify-between gap-3">
          <p className="text-xs text-text-muted">
            {t("agentRuntimes.hermesGuidance.restartHint")}
          </p>
          <Button size="sm" variant="outline" asChild>
            <a href={HERMES_INSTALL_DOCS_URL} target="_blank" rel="noopener noreferrer">
              <ExternalLink className="mr-1 h-3 w-3" />
              {t("agentRuntimes.hermesGuidance.docsLink")}
            </a>
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
