import { PanelShell } from "@/components/panels/PanelShell";
import { ChatPanel } from "@/features/chat/ChatPanel";
import { useTranslation } from "react-i18next";

export default function ChatView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.chat")}>
      <ChatPanel />
    </PanelShell>
  );
}
