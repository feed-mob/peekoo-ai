import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface SkillReplaceDialogProps {
  skillId: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}

export function SkillReplaceDialog({ skillId, onConfirm, onCancel }: SkillReplaceDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={!!skillId} onOpenChange={(open) => { if (!open) onCancel(); }}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("chatSettings.replaceSkillTitle")}</DialogTitle>
          <DialogDescription>
            {t("chatSettings.replaceSkillDescription", { skillId: skillId ?? "" })}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" size="sm" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button variant="destructive" size="sm" onClick={onConfirm}>
            {t("chatSettings.replaceSkillConfirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
