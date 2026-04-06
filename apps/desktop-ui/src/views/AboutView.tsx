import { PanelShell } from "@/components/panels/PanelShell";
import { AboutPanel } from "@/features/about/AboutPanel";
import { useTranslation } from "react-i18next";

export default function AboutView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.about")}>
      <AboutPanel />
    </PanelShell>
  );
}
