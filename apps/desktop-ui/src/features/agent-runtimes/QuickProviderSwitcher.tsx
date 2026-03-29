import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Badge } from "@/components/ui/badge";
import { ChevronDown, Star, AlertCircle } from "lucide-react";
import { type RuntimeInfo } from "@/types/agent-runtime";

interface QuickProviderSwitcherProps {
  providers: RuntimeInfo[];
  currentProvider: RuntimeInfo | null;
  currentModelDisplay?: string | null;
  onSwitch: (providerId: string) => void;
  onOpenSettings: () => void;
}

export function QuickProviderSwitcher({
  providers,
  currentProvider,
  currentModelDisplay,
  onSwitch,
  onOpenSettings,
}: QuickProviderSwitcherProps) {
  const [isOpen, setIsOpen] = useState(false);

  const installedProviders = providers.filter((p) => p.isInstalled && p.status === "ready");

  return (
    <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 gap-1 px-2 text-xs text-text-muted hover:text-text-primary"
        >
          {currentProvider ? (
            <>
              {currentProvider.isDefault && (
                <Star className="h-3 w-3 fill-primary text-primary" />
              )}
              <span className="max-w-[120px] truncate">{currentProvider.displayName}</span>
              {(currentModelDisplay || currentProvider.config.defaultModel) && (
                <span className="max-w-[180px] truncate text-text-muted">
                  • {currentModelDisplay ?? currentProvider.config.defaultModel}
                </span>
              )}
            </>
          ) : (
            <>
              <AlertCircle className="h-3 w-3 text-yellow-500" />
              <span>No runtime</span>
            </>
          )}
          <ChevronDown className="h-3 w-3" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="start" className="w-64">
        <DropdownMenuLabel>Switch Runtime</DropdownMenuLabel>
        <DropdownMenuSeparator />

        {installedProviders.length === 0 ? (
          <DropdownMenuItem disabled>No runtimes installed</DropdownMenuItem>
        ) : (
          installedProviders.map((provider) => (
            <DropdownMenuItem
              key={provider.providerId}
              onClick={() => {
                onSwitch(provider.providerId);
                setIsOpen(false);
              }}
              className="flex items-center justify-between"
            >
              <span className={provider.providerId === currentProvider?.providerId ? "font-medium" : ""}>
                {provider.displayName}
              </span>
              <div className="flex items-center gap-1">
                {provider.isDefault && (
                  <Badge variant="outline" className="h-4 px-1 text-[10px]">
                    Default
                  </Badge>
                )}
                {provider.providerId === currentProvider?.providerId && (
                  <div className="h-1.5 w-1.5 rounded-full bg-primary" />
                )}
              </div>
            </DropdownMenuItem>
          ))
        )}

        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={onOpenSettings} className="text-text-muted">
          Manage Runtimes...
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
